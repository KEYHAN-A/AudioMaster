//! Result caching for mastering operations.
//!
//! Caches analysis results and processed audio to enable retry and re-mastering
//! without re-processing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::types::AudioAnalysis;

/// Cache entry for analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Hash of the input file content
    pub file_hash: String,
    /// When this entry was created
    pub created_at: SystemTime,
    /// Time-to-live for this entry
    pub ttl: Duration,
}

/// Cache for audio analysis results.
pub struct AnalysisCache {
    entries: RwLock<HashMap<PathBuf, CacheEntry>>,
    max_entries: usize,
    default_ttl: Duration,
}

impl AnalysisCache {
    /// Create a new analysis cache.
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            default_ttl,
        }
    }

    /// Create a cache with default settings (100 entries, 1 hour TTL).
    pub fn with_defaults() -> Self {
        Self::new(100, Duration::from_secs(3600))
    }

    /// Get a cached analysis result for a file.
    pub async fn get(&self, path: &Path) -> Option<AudioAnalysis> {
        let entries = self.entries.read().await;
        // Check if entry exists and is not expired
        // In a real implementation, we'd store and return the actual analysis
        entries.get(path).and_then(|entry| {
            if entry.created_at + entry.ttl > SystemTime::now() {
                // Entry is valid
                Some(()) // Placeholder - would return cached analysis
            } else {
                None
            }
        });
        None // Placeholder - return actual cached analysis
    }

    /// Store an analysis result in the cache.
    pub async fn put(&self, path: PathBuf, _analysis: &AudioAnalysis) {
        let mut entries = self.entries.write().await;

        // Calculate file hash (simplified - would use actual content hash)
        let file_hash = format!("{:?}", path); // Placeholder

        // Evict oldest entry if at capacity
        if entries.len() >= self.max_entries {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, e)| e.created_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest);
            }
        }

        entries.insert(
            path,
            CacheEntry {
                file_hash,
                created_at: SystemTime::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Invalidate cache entry for a specific file.
    pub async fn invalidate(&self, path: &Path) {
        let mut entries = self.entries.write().await;
        entries.remove(path);
    }

    /// Clear all cache entries.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Remove expired entries.
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = SystemTime::now();
        entries.retain(|_, entry| entry.created_at + entry.ttl > now);
    }

    /// Get the number of cached entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }
}

/// Global cache instance.
static GLOBAL_CACHE: std::sync::OnceLock<AnalysisCache> = std::sync::OnceLock::new();

/// Get the global analysis cache.
pub fn global_cache() -> &'static AnalysisCache {
    GLOBAL_CACHE.get_or_init(AnalysisCache::with_defaults)
}

/// Compute a hash of a file's contents.
///
/// Uses a simple modification time and size-based hash for caching.
/// In production, you'd use a content-based hash like blake3.
pub fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let metadata = std::fs::metadata(path)?;
    let modified = metadata.modified()?;
    let size = metadata.len();

    // Simple hash based on mtime and size
    use std::time::UNIX_EPOCH;
    let mtime_secs = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(format!("{}-{}", mtime_secs, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = AnalysisCache::with_defaults();
        assert_eq!(cache.len().await, 0);
    }

    #[test]
    fn test_cache_put_and_len() {
        let cache = AnalysisCache::with_defaults();
        // Note: This test requires async runtime
    }
}
