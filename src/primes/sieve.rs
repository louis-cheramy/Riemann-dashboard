use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};

use super::store::MAGIC;

const SEGMENT_SIZE: u64 = 50_000_000;

pub struct SegmentProgress {
    pub segment: u32,
    pub start: u64,
    pub end: u64,
    pub count: u64,
}

fn simple_sieve(limit: u64) -> Vec<u64> {
    if limit < 2 {
        return Vec::new();
    }
    let limit = limit as usize;
    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    sieve[1] = false;
    let sqrt = (limit as f64).sqrt() as usize;
    for i in 2..=sqrt {
        if sieve[i] {
            let mut j = i * i;
            while j <= limit {
                sieve[j] = false;
                j += i;
            }
        }
    }
    sieve
        .into_iter()
        .enumerate()
        .filter_map(|(i, is_prime)| is_prime.then_some(i as u64))
        .collect()
}

/// Generate all primes up to `limit` and write them to `path`.
pub fn generate_primes(
    path: &Path,
    limit: u64,
    mut on_segment: impl FnMut(SegmentProgress),
) -> Result<u64> {
    anyhow::ensure!(limit >= 2, "La borne doit etre >= 2");

    let root = isqrt(limit) + 1;
    let base_primes = simple_sieve(root);

    if path.exists() {
        std::fs::remove_file(path).with_context(|| {
            format!(
                "impossible de remplacer {} (fichier peut-etre ouvert par l'application — relancez apres generation)",
                path.display()
            )
        })?;
    }

    let file = File::create(path).with_context(|| {
        format!(
            "creation de {} (verifiez les droits d'ecriture et que le fichier n'est pas ouvert ailleurs)",
            path.display()
        )
    })?;
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
    writer.write_all(MAGIC)?;

    for &p in &base_primes {
        writer.write_all(&p.to_le_bytes())?;
    }

    let mut total = base_primes.len() as u64;
    let mut start = root + 1;
    let mut segment = 0u32;

    while start <= limit {
        let end = (start + SEGMENT_SIZE).min(limit + 1);
        let len = (end - start) as usize;
        let mut segment_sieve = vec![true; len];

        for &p in &base_primes {
            let p = p as u64;
            let mut m = p * p;
            if m < start {
                m = ((start + p - 1) / p) * p;
            }
            while m < end {
                segment_sieve[(m - start) as usize] = false;
                m += p;
            }
        }

        let mut count = 0u64;
        for (offset, &is_prime) in segment_sieve.iter().enumerate() {
            if is_prime {
                let value = start + offset as u64;
                writer.write_all(&value.to_le_bytes())?;
                count += 1;
            }
        }
        total += count;

        segment += 1;
        on_segment(SegmentProgress {
            segment,
            start,
            end: end - 1,
            count,
        });

        start = end;
    }

    writer.flush()?;
    Ok(total)
}

fn isqrt(n: u64) -> u64 {
    (n as f64).sqrt() as u64
}
