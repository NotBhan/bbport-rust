//! Shader/pipeline blob cache, ported from
//! `shadps4/video_core/cache_storage.{h,cpp}`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// `BlobType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BlobType {
    ShaderMeta = 0,
    ShaderBinary = 1,
    PipelineKey = 2,
    ShaderProfile = 3,
}

impl BlobType {
    pub const ALL: [BlobType; 4] = [
        Self::ShaderMeta,
        Self::ShaderBinary,
        Self::PipelineKey,
        Self::ShaderProfile,
    ];

    fn extension(self) -> &'static str {
        match self {
            Self::ShaderMeta => "meta",
            Self::ShaderBinary => "spv",
            Self::PipelineKey => "key",
            Self::ShaderProfile => "profile",
        }
    }
}

/// `DataBase`.
#[derive(Debug, Default)]
pub struct DataBase {
    cache_path: PathBuf,
    opened: bool,
}

impl DataBase {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            cache_path: path.into(),
            opened: false,
        }
    }

    pub fn open(&mut self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.cache_path)?;
        self.opened = true;
        Ok(())
    }

    pub fn close(&mut self) {
        self.opened = false;
    }

    pub fn is_opened(&self) -> bool {
        self.opened
    }

    pub fn finish_preload(&mut self) {}

    fn dir(&self, kind: BlobType) -> PathBuf {
        self.cache_path.join(kind.extension())
    }

    fn file(&self, kind: BlobType, name: &str) -> PathBuf {
        self.dir(kind).join(format!("{name}.{}", kind.extension()))
    }

    /// `Clear`.
    pub fn clear(&mut self) -> std::io::Result<()> {
        for kind in BlobType::ALL {
            let _ = std::fs::remove_dir_all(self.dir(kind));
        }
        Ok(())
    }

    /// `ClearType`.
    pub fn clear_type(&mut self, kind: BlobType) -> std::io::Result<()> {
        std::fs::remove_dir_all(self.dir(kind))
    }

    /// `Seed`; copies catalog blobs the cache does not have yet.
    pub fn seed(&mut self, catalog: &Path) -> std::io::Result<u32> {
        let mut added = 0;
        for kind in [BlobType::ShaderMeta, BlobType::PipelineKey] {
            let source = catalog.join(kind.extension());
            let Ok(entries) = std::fs::read_dir(&source) else {
                continue;
            };
            let target = self.dir(kind);
            std::fs::create_dir_all(&target)?;
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
                    continue;
                };
                if !self.has(kind, name) {
                    std::fs::copy(&path, self.file(kind, name))?;
                    added += 1;
                }
            }
        }
        Ok(added)
    }

    /// `Has`.
    pub fn has(&self, kind: BlobType, name: &str) -> bool {
        self.file(kind, name).is_file()
    }

    /// `Count`.
    pub fn count(&self, kind: BlobType) -> u32 {
        std::fs::read_dir(self.dir(kind))
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.path().is_file())
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// `Save` bytes.
    pub fn save_bytes(&mut self, kind: BlobType, name: &str, data: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(self.dir(kind))?;
        std::fs::write(self.file(kind, name), data)
    }

    /// `Save` words.
    pub fn save_words(&mut self, kind: BlobType, name: &str, data: &[u32]) -> std::io::Result<()> {
        let bytes: Vec<u8> = data.iter().flat_map(|word| word.to_le_bytes()).collect();
        self.save_bytes(kind, name, &bytes)
    }

    /// `Load` bytes.
    pub fn load_bytes(&self, kind: BlobType, name: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.file(kind, name))
    }

    /// `Load` words.
    pub fn load_words(&self, kind: BlobType, name: &str) -> std::io::Result<Vec<u32>> {
        Ok(self
            .load_bytes(kind, name)?
            .chunks(4)
            .filter(|word| word.len() == 4)
            .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
            .collect())
    }

    /// `ForEachBlob`.
    pub fn for_each_blob(&self, kind: BlobType, mut func: impl FnMut(Vec<u8>)) {
        let Ok(entries) = std::fs::read_dir(self.dir(kind)) else {
            return;
        };
        let mut seen = HashSet::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if seen.insert(path.clone()) {
                if let Ok(data) = std::fs::read(path) {
                    func(data);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_database_saves_loads_counts_and_seeds() {
        let root = std::env::temp_dir().join(format!("bbport-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut db = DataBase::new(root.join("cache"));
        db.open().unwrap();
        assert!(db.is_opened());
        db.save_words(BlobType::ShaderBinary, "a", &[1, 2, 3])
            .unwrap();
        assert!(db.has(BlobType::ShaderBinary, "a"));
        assert_eq!(
            db.load_words(BlobType::ShaderBinary, "a").unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(db.count(BlobType::ShaderBinary), 1);

        let catalog = root.join("catalog");
        std::fs::create_dir_all(catalog.join("meta")).unwrap();
        std::fs::write(catalog.join("meta/b.meta"), [9]).unwrap();
        assert_eq!(db.seed(&catalog).unwrap(), 1);
        assert!(db.has(BlobType::ShaderMeta, "b"));
        let mut seen = 0;
        db.for_each_blob(BlobType::ShaderBinary, |_| seen += 1);
        assert_eq!(seen, 1);
        db.clear_type(BlobType::ShaderMeta).unwrap();
        assert_eq!(db.count(BlobType::ShaderMeta), 0);
        db.close();
        let _ = std::fs::remove_dir_all(root);
    }
}
