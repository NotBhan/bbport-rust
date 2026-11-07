//! PNG images through the vendored `stb_image`/`stb_image_write`, ported from
//! `legacy/gpu/shadps4/common/stb.h` and `stb.cpp`.
//!
//! The C build compiles stb with `STBI_ONLY_PNG` and `STBI_NO_STDIO`; the same surface is
//! implemented here on top of `flate2`, so both sides only ever handle PNG from memory.

use std::io::{Read, Write};

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// 1 grey, 2 grey+alpha, 3 RGB, 4 RGBA.
    pub channels: u32,
    pub data: Vec<u8>,
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The five filter types the PNG spec defines.
fn unfilter(filter: u8, bpp: usize, previous: &[u8], current: &mut [u8]) -> bool {
    let stride = current.len();
    match filter {
        0 => {}
        1 => {
            for index in bpp..stride {
                current[index] = current[index].wrapping_add(current[index - bpp]);
            }
        }
        2 => {
            for index in 0..stride {
                current[index] = current[index].wrapping_add(previous[index]);
            }
        }
        3 => {
            for index in 0..stride {
                let left = if index >= bpp {
                    current[index - bpp]
                } else {
                    0
                };
                let up = previous[index];
                current[index] =
                    current[index].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8);
            }
        }
        4 => {
            for index in 0..stride {
                let left = if index >= bpp {
                    current[index - bpp]
                } else {
                    0
                };
                let up = previous[index];
                let up_left = if index >= bpp {
                    previous[index - bpp]
                } else {
                    0
                };
                let predictor = paeth(left, up, up_left);
                current[index] = current[index].wrapping_add(predictor);
            }
        }
        _ => return false,
    }
    true
}

fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let (left, up, up_left) = (i32::from(left), i32::from(up), i32::from(up_left));
    let estimate = left + up - up_left;
    let distances = [
        (estimate - left).abs(),
        (estimate - up).abs(),
        (estimate - up_left).abs(),
    ];
    if distances[0] <= distances[1] && distances[0] <= distances[2] {
        left as u8
    } else if distances[1] <= distances[2] {
        up as u8
    } else {
        up_left as u8
    }
}

/// `stbi_load_from_memory` for PNG.
pub fn load_png(bytes: &[u8]) -> Option<Image> {
    if bytes.len() < 8 || bytes[0..8] != SIGNATURE {
        return None;
    }
    let mut offset = 8;
    let mut header: Option<(u32, u32, u32)> = None;
    let mut compressed = Vec::new();
    while offset + 8 <= bytes.len() {
        let length = be32(&bytes[offset..]) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let start = offset + 8;
        let end = start.checked_add(length)?;
        if end + 4 > bytes.len() {
            return None;
        }
        let body = &bytes[start..end];
        // The chunk CRC covers the kind and the body; a corrupt pack must not decode.
        let expected = be32(&bytes[end..]);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(kind);
        hasher.update(body);
        if hasher.finalize() != expected {
            return None;
        }
        match kind {
            b"IHDR" => {
                if body.len() != 13 {
                    return None;
                }
                let width = be32(body);
                let height = be32(&body[4..]);
                let depth = body[8];
                let color = body[9];
                let interlace = body[12];
                if width == 0 || height == 0 || depth != 8 || interlace != 0 {
                    return None;
                }
                let channels = match color {
                    0 => 1,
                    2 => 3,
                    4 => 2,
                    6 => 4,
                    _ => return None,
                };
                header = Some((width, height, channels));
            }
            b"IDAT" => compressed.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        offset = end + 4;
    }
    let (width, height, channels) = header?;
    if compressed.is_empty() {
        return None;
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(compressed.as_slice())
        .read_to_end(&mut raw)
        .ok()?;
    let stride = width as usize * channels as usize;
    if raw.len() != (stride + 1) * height as usize {
        return None;
    }
    let mut data = vec![0u8; stride * height as usize];
    let bpp = channels as usize;
    for row in 0..height as usize {
        let source = &raw[row * (stride + 1)..];
        let filter = source[0];
        let mut current = vec![0u8; stride];
        current.copy_from_slice(&source[1..=stride]);
        let previous = if row == 0 {
            vec![0u8; stride]
        } else {
            data[(row - 1) * stride..row * stride].to_vec()
        };
        if !unfilter(filter, bpp, &previous, &mut current) {
            return None;
        }
        data[row * stride..(row + 1) * stride].copy_from_slice(&current);
    }
    Some(Image {
        width,
        height,
        channels,
        data,
    })
}

/// `stbi_write_png_to_mem`: 8-bit PNG, no interlacing, filter type none.
pub fn write_png(width: u32, height: u32, channels: u32, data: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || !(1..=4).contains(&channels) {
        return None;
    }
    let stride = width as usize * channels as usize;
    if data.len() < stride * height as usize {
        return None;
    }
    let color = match channels {
        1 => 0,
        2 => 4,
        3 => 2,
        4 => 6,
        _ => return None,
    };
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&data[row * stride..(row + 1) * stride]);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw).ok()?;
    let compressed = encoder.finish().ok()?;
    let mut out = Vec::new();
    out.extend_from_slice(&SIGNATURE);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);
    write_chunk(&mut out, b"IDAT", &compressed);
    write_chunk(&mut out, b"IEND", &[]);
    Some(out)
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(kind);
    hasher.update(body);
    out.extend_from_slice(&hasher.finalize().to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_survives_a_write_and_load_round_trip() {
        let pixels: Vec<u8> = (0..(4 * 3 * 3)).map(|value| value as u8).collect();
        let png = write_png(4, 3, 3, &pixels).unwrap();
        let image = load_png(&png).unwrap();
        assert_eq!((image.width, image.height, image.channels), (4, 3, 3));
        assert_eq!(image.data, pixels);
        // A grey+alpha image uses colour type 4 and keeps two channels.
        let grey: Vec<u8> = (0..8).collect();
        let png = write_png(2, 2, 2, &grey).unwrap();
        assert_eq!(load_png(&png).unwrap().channels, 2);
    }

    #[test]
    fn a_filtered_row_unfilters_like_sub() {
        // Filter 1 (Sub) on a 2x1 RGB image: each byte adds the byte three positions to its left.
        let raw = [1u8, 10, 5, 5, 20, 25, 30];
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut png = Vec::from(SIGNATURE);
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        write_chunk(&mut png, b"IHDR", &ihdr);
        write_chunk(&mut png, b"IDAT", &compressed);
        write_chunk(&mut png, b"IEND", &[]);
        let image = load_png(&png).unwrap();
        assert_eq!(image.data, vec![10, 5, 5, 30, 30, 35]);
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert!(load_png(b"not a png").is_none());
        let mut png = write_png(2, 2, 3, &[0u8; 12]).unwrap();
        // A flipped payload byte must fail the chunk CRC.
        let last = png.len() - 20;
        png[last] ^= 0xff;
        assert!(load_png(&png).is_none());
        assert!(write_png(0, 2, 3, &[]).is_none());
        assert!(write_png(2, 2, 3, &[0u8; 4]).is_none());
    }
}
