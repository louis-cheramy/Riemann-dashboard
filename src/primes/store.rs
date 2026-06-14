use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use memmap2::Mmap;

pub const MAGIC: &[u8; 8] = b"PRIMEV2\x00";
pub const DEFAULT_PRIME_FILE: &str = "nombres_premiers.bin";

pub struct PrimeStore {
    _file: File,
    mmap: Mmap,
    data_offset: usize,
}

impl PrimeStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).with_context(|| format!("ouverture de {}", path.display()))?;
        let mmap = unsafe { Mmap::map(&file)? };

        let data_offset = if mmap.len() >= 8 && &mmap[..8] == MAGIC {
            8
        } else {
            0
        };

        let payload = mmap.len() - data_offset;
        if data_offset == 8 {
            if payload % 8 != 0 {
                bail!("Fichier PRIMEV2 invalide (taille non multiple de 8)");
            }
        } else if payload % 4 == 0 {
            // legacy uint32
        } else {
            bail!("Format de fichier inconnu");
        }

        Ok(Self {
            _file: file,
            mmap,
            data_offset,
        })
    }

    pub fn default_path() -> PathBuf {
        resolve_prime_path()
    }

    /// Cherche le fichier .bin (dossier courant, exe, racine projet).
    pub fn count(&self) -> u64 {
        if self.is_legacy() {
            ((self.mmap.len() - self.data_offset) / 4) as u64
        } else {
            ((self.mmap.len() - self.data_offset) / 8) as u64
        }
    }

    pub fn is_legacy(&self) -> bool {
        self.data_offset == 0
    }

    pub fn first(&self) -> Option<u64> {
        self.at(0)
    }

    pub fn last(&self) -> Option<u64> {
        let n = self.count();
        if n == 0 {
            None
        } else {
            self.at(n - 1)
        }
    }

    pub fn at(&self, index: u64) -> Option<u64> {
        if index >= self.count() {
            return None;
        }
        Some(if self.is_legacy() {
            let offset = self.data_offset + index as usize * 4;
            u32::from_le_bytes(self.mmap[offset..offset + 4].try_into().unwrap()) as u64
        } else {
            let offset = self.data_offset + index as usize * 8;
            u64::from_le_bytes(self.mmap[offset..offset + 8].try_into().unwrap())
        })
    }

    /// Primes in [lo, hi] via binary search on the sorted file.
    pub fn range_indices(&self, lo: u64, hi: u64) -> (u64, u64) {
        let count = self.count();
        if count == 0 {
            return (0, 0);
        }
        let start = self.partition_left(lo);
        let end = self.partition_right(hi.min(u64::MAX));
        (start.min(count), end.min(count))
    }

    pub fn range_len(&self, lo: u64, hi: u64) -> u64 {
        let (a, b) = self.range_indices(lo, hi);
        b.saturating_sub(a)
    }

    pub fn for_each_in_range(&self, lo: u64, hi: u64, mut f: impl FnMut(u64)) {
        let (start, end) = self.range_indices(lo, hi);
        for i in start..end {
            if let Some(p) = self.at(i) {
                f(p);
            }
        }
    }

    pub fn collect_range(&self, lo: u64, hi: u64, max_points: usize) -> Vec<u64> {
        let (start, end) = self.range_indices(lo, hi);
        let len = (end - start) as usize;
        if len <= max_points {
            (start..end).filter_map(|i| self.at(i)).collect()
        } else {
            // subsample evenly for plotting huge intervals
            let step = len as f64 / max_points as f64;
            (0..max_points)
                .filter_map(|k| {
                    let idx = start + (k as f64 * step) as u64;
                    self.at(idx)
                })
                .collect()
        }
    }

    fn partition_left(&self, target: u64) -> u64 {
        let mut lo = 0u64;
        let mut hi = self.count();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.at(mid) {
                Some(v) if v < target => lo = mid + 1,
                Some(_) => hi = mid,
                None => break,
            }
        }
        lo
    }

    fn partition_right(&self, target: u64) -> u64 {
        let mut lo = 0u64;
        let mut hi = self.count();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.at(mid) {
                Some(v) if v <= target => lo = mid + 1,
                Some(_) => hi = mid,
                None => break,
            }
        }
        lo
    }
}

/// Emplacement du fichier binaire pour lecture/ecriture.
pub fn resolve_prime_path() -> PathBuf {
    for path in search_paths() {
        if path.exists() {
            return path;
        }
    }
    default_output_path()
}

fn search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(DEFAULT_PRIME_FILE));
    }
    paths.push(PathBuf::from(DEFAULT_PRIME_FILE));

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            paths.push(exe_dir.join(DEFAULT_PRIME_FILE));
            if let Some(target_dir) = exe_dir.parent() {
                if let Some(project_dir) = target_dir.parent() {
                    paths.push(project_dir.join(DEFAULT_PRIME_FILE));
                }
            }
        }
    }

    paths
}

fn default_output_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if exe_dir.ends_with("release") || exe_dir.ends_with("debug") {
                if let Some(target_dir) = exe_dir.parent() {
                    if target_dir.file_name().and_then(|n| n.to_str()) == Some("target") {
                        if let Some(project_dir) = target_dir.parent() {
                            return project_dir.join(DEFAULT_PRIME_FILE);
                        }
                    }
                }
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_PRIME_FILE)
}
