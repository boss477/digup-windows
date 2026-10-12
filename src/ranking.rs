//! Query-local modality calibration. Scores are relative relevance, not probabilities.
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Modality {
    Text,
    Image,
    Audio,
    Video,
}
fn modality(path: &std::path::Path) -> Modality {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" | "png" | "webp" | "heic" | "bmp" | "tif" | "tiff" => Modality::Image,
        "wav" | "mp3" | "flac" | "m4a" | "ogg" | "aac" => Modality::Audio,
        "mp4" | "mkv" | "mov" | "webm" | "avi" => Modality::Video,
        _ => Modality::Text,
    }
}

/// Standardize each modality's query-score distribution before global top-k.
/// Apply only with enough examples in every group; never equalize file-count quotas.
/// Inputs must include the full candidate pool, not already-truncated top results.
pub fn calibrate(results: &mut [(PathBuf, f64)]) -> bool {
    let mut groups: BTreeMap<Modality, Vec<f64>> = BTreeMap::new();
    for (path, score) in results.iter() {
        if !score.is_finite() {
            return false;
        }
        groups.entry(modality(path)).or_default().push(*score);
    }
    if groups.len() < 2 || groups.values().any(|scores| scores.len() < 32) {
        return false;
    }
    let mut stats = BTreeMap::new();
    for (kind, scores) in groups {
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        // Degenerate distributions cannot support a meaningful scale estimate.
        if variance < 1e-8 {
            return false;
        }
        stats.insert(kind, (mean, variance.sqrt()));
    }
    for (path, score) in results {
        let (mean, deviation) = stats[&modality(path)];
        *score = (*score - mean) / deviation;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<(PathBuf, f64)> {
        (0..40)
            .flat_map(|i| {
                let relative = i as f64 / 100.;
                [
                    (PathBuf::from(format!("{i}.png")), 0.55 + relative),
                    (PathBuf::from(format!("{i}.txt")), 0.10 + relative),
                ]
            })
            .collect()
    }
    #[test]
    fn removes_uniform_modality_offset_and_preserves_within_group_order() {
        let mut results = fixture();
        assert!(calibrate(&mut results));
        for pair in results.as_chunks::<2>().0 {
            assert!((pair[0].1 - pair[1].1).abs() < 1e-10);
        }
        assert!(results[78].1 > results[0].1);
    }
    #[test]
    fn small_or_single_modality_search_is_unchanged() {
        let mut small = fixture()[..10].to_vec();
        let original = small.clone();
        assert!(!calibrate(&mut small));
        assert_eq!(small, original);
        let mut text: Vec<_> = fixture()
            .into_iter()
            .filter(|(p, _)| modality(p) == Modality::Text)
            .collect();
        let original = text.clone();
        assert!(!calibrate(&mut text));
        assert_eq!(text, original);
    }
    #[test]
    fn degenerate_group_falls_back_without_partial_changes() {
        let mut results = fixture();
        for (p, s) in &mut results {
            if modality(p) == Modality::Text {
                *s = 0.5;
            }
        }
        let original = results.clone();
        assert!(!calibrate(&mut results));
        assert_eq!(results, original);
    }
}
