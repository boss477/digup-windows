use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Cache(Connection);
impl Cache {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS embeddings (key TEXT PRIMARY KEY, vector TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self(connection))
    }
    // Include exact input and provider identity so changed content or models never reuse old vectors.
    pub fn key(url: &str, model: &str, input: &str) -> String {
        let mut digest = Sha256::new();
        for value in [url, model, input] {
            digest.update(value.len().to_le_bytes());
            digest.update(value.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }
    pub fn get(&self, key: &str) -> Result<Option<Vec<f64>>, String> {
        let value: Option<String> = self
            .0
            .query_row("SELECT vector FROM embeddings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?;
        value
            .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn put(&self, key: &str, vector: &[f64]) -> Result<(), String> {
        let value = serde_json::to_string(vector).map_err(|e| e.to_string())?;
        self.0
            .execute(
                "INSERT OR REPLACE INTO embeddings VALUES (?1,?2)",
                params![key, value],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
pub fn chunks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + 2400).min(chars.len());
        result.push(chars[start..end].iter().collect());
        if end == chars.len() {
            break;
        }
        start = end - 240;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_include_tail_without_splitting_unicode() {
        let text = "猫".repeat(5000);
        let pieces = chunks(&text);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces.last().unwrap().chars().count(), 680);
        assert!(chunks("").is_empty());
    }
    #[test]
    fn cache_separates_models_and_roundtrips_vectors() {
        let cache = Cache(Connection::open_in_memory().unwrap());
        cache
            .0
            .execute_batch("CREATE TABLE embeddings (key TEXT PRIMARY KEY, vector TEXT NOT NULL)")
            .unwrap();
        let key = Cache::key("local", "model1", "text");
        assert_ne!(key, Cache::key("local", "model2", "text"));
        assert!(cache.get(&key).unwrap().is_none());
        cache.put(&key, &[1., 2.]).unwrap();
        assert_eq!(cache.get(&key).unwrap().unwrap(), vec![1., 2.]);
    }
}
