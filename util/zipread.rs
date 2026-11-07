//! Minimal ZIP reader (stored + deflate, Zip64) for the PKG setup and co-op package
//! installers. Central-directory parsing follows PKWARE APPNOTE 4.4.6; entries are streamed
//! to a writer so multi-gigabyte PKGs never sit in memory.

use flate2::read::DeflateDecoder;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_EXTRA_ID: u16 = 0x0001;
const MAX_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: u64 = 1_000_000;

#[derive(Debug, Error)]
pub enum ZipError {
    #[error("not a ZIP archive (end of central directory not found)")]
    NotZip,
    #[error("truncated ZIP structure")]
    Truncated,
    #[error("ZIP structure too large: {0}")]
    TooLarge(&'static str),
    #[error("unsupported ZIP compression method {0}")]
    UnsupportedMethod(u16),
    #[error("{name} is damaged (CRC {expected:08x}, got {actual:08x})")]
    Crc {
        name: String,
        expected: u32,
        actual: u32,
    },
    #[error("{0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub method: u16,
    pub crc32: u32,
    pub local_offset: u64,
    pub is_dir: bool,
}

pub struct ZipArchive {
    file: File,
    entries: Vec<ZipEntry>,
}

impl ZipArchive {
    pub fn open(path: &Path) -> Result<Self, ZipError> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        let tail_length = length.min(66_000) as usize;
        let mut tail = vec![0u8; tail_length];
        file.seek(SeekFrom::Start(length - tail_length as u64))?;
        file.read_exact(&mut tail)?;
        let mut eocd = None;
        for index in (0..tail.len().saturating_sub(21)).rev() {
            if u32::from_le_bytes(tail[index..index + 4].try_into().unwrap()) == EOCD_SIGNATURE {
                eocd = Some(index);
                break;
            }
        }
        let eocd = eocd.ok_or(ZipError::NotZip)?;
        let eocd_absolute = length - tail_length as u64 + eocd as u64;
        let read_u16 = |at: usize| u16::from_le_bytes(tail[at..at + 2].try_into().unwrap()) as u64;
        let read_u32 = |at: usize| u32::from_le_bytes(tail[at..at + 4].try_into().unwrap()) as u64;
        let mut total_entries = read_u16(eocd + 10);
        let mut directory_size = read_u32(eocd + 12);
        let mut directory_offset = read_u32(eocd + 16);

        if total_entries == 0xffff
            || directory_offset == 0xffff_ffff
            || directory_size == 0xffff_ffff
        {
            let locator_at = eocd_absolute.checked_sub(20).ok_or(ZipError::Truncated)? as usize
                - (length - tail_length as u64) as usize;
            if locator_at + 20 <= tail.len()
                && read_u32(locator_at) == ZIP64_LOCATOR_SIGNATURE as u64
            {
                let eocd64 =
                    u64::from_le_bytes(tail[locator_at + 8..locator_at + 16].try_into().unwrap());
                let mut header = [0u8; 56];
                file.seek(SeekFrom::Start(eocd64))?;
                file.read_exact(&mut header)?;
                if u32::from_le_bytes(header[0..4].try_into().unwrap()) != ZIP64_EOCD_SIGNATURE {
                    return Err(ZipError::Truncated);
                }
                total_entries = u64::from_le_bytes(header[32..40].try_into().unwrap());
                directory_size = u64::from_le_bytes(header[40..48].try_into().unwrap());
                directory_offset = u64::from_le_bytes(header[48..56].try_into().unwrap());
            }
        }

        if total_entries > MAX_ENTRIES {
            return Err(ZipError::TooLarge("entry count"));
        }
        if directory_size > MAX_DIRECTORY_BYTES {
            return Err(ZipError::TooLarge("central directory"));
        }
        if directory_offset > length || directory_size > length - directory_offset {
            return Err(ZipError::Truncated);
        }
        let directory_length =
            usize::try_from(directory_size).map_err(|_| ZipError::TooLarge("central directory"))?;
        let mut directory = vec![0u8; directory_length];
        file.seek(SeekFrom::Start(directory_offset))?;
        file.read_exact(&mut directory)?;
        let entries = parse_directory(&directory, total_entries)?;
        Ok(Self { file, entries })
    }

    pub fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    pub fn by_name(&self, name: &str) -> Option<&ZipEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    /// Stream one entry to `writer`, verifying its CRC when the archive records one.
    pub fn extract_to<W: Write>(&self, entry: &ZipEntry, writer: &mut W) -> Result<u64, ZipError> {
        let mut file = self.file.try_clone()?;
        file.seek(SeekFrom::Start(entry.local_offset))?;
        let mut header = [0u8; 30];
        file.read_exact(&mut header)?;
        if u32::from_le_bytes(header[0..4].try_into().unwrap()) != LOCAL_SIGNATURE {
            return Err(ZipError::Truncated);
        }
        let name_length = u16::from_le_bytes(header[26..28].try_into().unwrap()) as u64;
        let extra_length = u16::from_le_bytes(header[28..30].try_into().unwrap()) as u64;
        let data_offset = entry
            .local_offset
            .checked_add(30)
            .and_then(|offset| offset.checked_add(name_length))
            .and_then(|offset| offset.checked_add(extra_length))
            .ok_or(ZipError::Truncated)?;
        file.seek(SeekFrom::Start(data_offset))?;

        let mut hasher = crc32fast::Hasher::new();
        let mut written = 0u64;
        let count = |hasher: &mut crc32fast::Hasher, written: &mut u64, chunk: &[u8]| {
            hasher.update(chunk);
            *written += chunk.len() as u64;
        };
        match entry.method {
            0 => {
                let mut remaining = entry.compressed_size;
                let mut buffer = [0u8; 64 * 1024];
                while remaining > 0 {
                    let want = buffer.len().min(remaining as usize);
                    let read = file.read(&mut buffer[..want])?;
                    if read == 0 {
                        return Err(ZipError::Truncated);
                    }
                    writer.write_all(&buffer[..read])?;
                    count(&mut hasher, &mut written, &buffer[..read]);
                    remaining -= read as u64;
                }
            }
            8 => {
                let mut decoder = DeflateDecoder::new(file.take(entry.compressed_size));
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    let read = decoder.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    writer.write_all(&buffer[..read])?;
                    count(&mut hasher, &mut written, &buffer[..read]);
                }
            }
            other => return Err(ZipError::UnsupportedMethod(other)),
        }
        let actual = hasher.finalize();
        if entry.crc32 != 0 && actual != entry.crc32 {
            return Err(ZipError::Crc {
                name: entry.name.clone(),
                expected: entry.crc32,
                actual,
            });
        }
        Ok(written)
    }
}

fn parse_directory(data: &[u8], total_entries: u64) -> Result<Vec<ZipEntry>, ZipError> {
    let mut entries = Vec::new();
    let mut at = 0usize;
    while entries.len() < total_entries as usize && at + 46 <= data.len() {
        if u32::from_le_bytes(data[at..at + 4].try_into().unwrap()) != CENTRAL_SIGNATURE {
            break;
        }
        let method = u16::from_le_bytes(data[at + 10..at + 12].try_into().unwrap());
        let crc32 = u32::from_le_bytes(data[at + 16..at + 20].try_into().unwrap());
        let mut compressed_size =
            u32::from_le_bytes(data[at + 20..at + 24].try_into().unwrap()) as u64;
        let mut size = u32::from_le_bytes(data[at + 24..at + 28].try_into().unwrap()) as u64;
        let name_length = u16::from_le_bytes(data[at + 28..at + 30].try_into().unwrap()) as usize;
        let extra_length = u16::from_le_bytes(data[at + 30..at + 32].try_into().unwrap()) as usize;
        let comment_length =
            u16::from_le_bytes(data[at + 32..at + 34].try_into().unwrap()) as usize;
        let mut local_offset =
            u32::from_le_bytes(data[at + 42..at + 46].try_into().unwrap()) as u64;
        let name_start = at + 46;
        let name_end = name_start
            .checked_add(name_length)
            .ok_or(ZipError::Truncated)?;
        let extra_end = name_end
            .checked_add(extra_length)
            .ok_or(ZipError::Truncated)?;
        let next = extra_end
            .checked_add(comment_length)
            .ok_or(ZipError::Truncated)?;
        if next > data.len() {
            return Err(ZipError::Truncated);
        }
        let name = String::from_utf8_lossy(&data[name_start..name_end]).into_owned();
        if size == 0xffff_ffff || compressed_size == 0xffff_ffff || local_offset == 0xffff_ffff {
            let mut extra_at = name_end;
            while extra_at + 4 <= extra_end {
                let id = u16::from_le_bytes(data[extra_at..extra_at + 2].try_into().unwrap());
                let field_size =
                    u16::from_le_bytes(data[extra_at + 2..extra_at + 4].try_into().unwrap())
                        as usize;
                let field_start = extra_at + 4;
                let field_end = field_start
                    .checked_add(field_size)
                    .ok_or(ZipError::Truncated)?;
                if field_end > data.len() {
                    break;
                }
                if id == ZIP64_EXTRA_ID {
                    let mut cursor = field_start;
                    let take_u64 = |cursor: &mut usize| -> Result<u64, ZipError> {
                        if *cursor + 8 > field_end {
                            return Err(ZipError::Truncated);
                        }
                        let value =
                            u64::from_le_bytes(data[*cursor..*cursor + 8].try_into().unwrap());
                        *cursor += 8;
                        Ok(value)
                    };
                    if size == 0xffff_ffff {
                        size = take_u64(&mut cursor)?;
                    }
                    if compressed_size == 0xffff_ffff {
                        compressed_size = take_u64(&mut cursor)?;
                    }
                    if local_offset == 0xffff_ffff {
                        local_offset = take_u64(&mut cursor)?;
                    }
                    break;
                }
                extra_at = field_end;
            }
        }
        entries.push(ZipEntry {
            is_dir: name.ends_with('/'),
            name,
            size,
            compressed_size,
            method,
            crc32,
            local_offset,
        });
        at = next;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-zip-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build a stored-only ZIP with the given (name, contents) entries.
    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, contents) in entries {
            let offset = out.len() as u32;
            let crc = crc32fast::hash(contents);
            out.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            out.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(contents);

            central.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            central.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let central_offset = out.len() as u32;
        let central_size = central.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&central_size.to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn lists_and_extracts_stored_entries() {
        let dir = temp_dir("stored");
        let path = dir.join("test.zip");
        std::fs::write(
            &path,
            build_zip(&[("a/b.txt", b"hello"), ("c.bin", b"world!")]),
        )
        .unwrap();
        let archive = ZipArchive::open(&path).unwrap();
        assert_eq!(archive.entries().len(), 2);
        let entry = archive.by_name("a/b.txt").unwrap();
        assert_eq!(entry.size, 5);
        let mut extracted = Vec::new();
        archive.extract_to(entry, &mut extracted).unwrap();
        assert_eq!(extracted, b"hello");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_corrupt_crc_is_reported() {
        let dir = temp_dir("crc");
        let path = dir.join("test.zip");
        let mut data = build_zip(&[("a.txt", b"hello")]);
        // Corrupt the stored bytes inside the local header data.
        let index = data
            .windows(5)
            .position(|window| window == b"hello")
            .unwrap();
        data[index] = b'x';
        std::fs::write(&path, data).unwrap();
        let archive = ZipArchive::open(&path).unwrap();
        let entry = archive.by_name("a.txt").unwrap();
        let mut out = Vec::new();
        let error = archive.extract_to(entry, &mut out).unwrap_err();
        assert!(matches!(error, ZipError::Crc { .. }));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_oversized_directory_is_rejected_before_allocating() {
        let dir = temp_dir("oversized-directory");
        let path = dir.join("huge.zip");
        let mut data = build_zip(&[("a.txt", b"hello")]);
        let eocd = data.len() - 22;
        data[eocd + 12..eocd + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&path, data).unwrap();
        assert!(matches!(
            ZipArchive::open(&path),
            Err(ZipError::TooLarge("central directory"))
        ));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_non_zip_file_is_rejected() {
        let dir = temp_dir("notzip");
        let path = dir.join("test.zip");
        std::fs::write(&path, b"definitely not a zip").unwrap();
        assert!(matches!(ZipArchive::open(&path), Err(ZipError::NotZip)));
        let _ = std::fs::remove_dir_all(dir);
    }
}
