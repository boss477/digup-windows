use crate::{documents, Entry, SearchOutput};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn search(
    entries: &[Entry],
    query: &str,
    cancel: &AtomicBool,
    include_code: bool,
    include_ocr: bool,
) -> Result<SearchOutput, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err("Enter text to search for".into());
    }
    let mut output = SearchOutput {
        results: vec![],
        warnings: vec![],
        locations: Default::default(),
    };
    for entry in entries.iter().filter(|e| {
        !e.directory
            && (documents::supported(&e.path)
                || (include_code && documents::source_file(&e.path))
                || (include_ocr && crate::media::is_image(&e.path)))
    }) {
        if cancel.load(Ordering::Relaxed) {
            return Err("Search cancelled".into());
        }
        match documents::sections(&entry.path, include_ocr, cancel) {
            Ok(sections) => {
                if let Some(section) = sections
                    .iter()
                    .find(|section| section.text.to_lowercase().contains(&query))
                {
                    output.results.push((entry.path.clone(), 1.0));
                    let line = section
                        .text
                        .lines()
                        .find(|line| line.to_lowercase().contains(&query))
                        .unwrap_or(&section.text);
                    output.locations.insert(
                        entry.path.clone(),
                        crate::location::Match {
                            page: section.page,
                            seconds: None,
                            snippet: line.chars().take(300).collect(),
                        },
                    );
                }
            }
            Err(error) => output
                .warnings
                .push(format!("{}: {error}", entry.path.display())),
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Search cancelled".into());
    }
    output.results.sort_by(|a, b| a.0.cmp(&b.0));
    output.results.truncate(500);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires Tesseract, Poppler and generated OCR fixtures"]
    fn ocr_fixture_search() {
        let root = std::path::PathBuf::from(std::env::var("DIGUP_OCR_FIXTURE_ROOT").unwrap());
        let entries: Vec<_> = ["ocr.png", "scanned.pdf"]
            .iter()
            .map(|name| Entry {
                path: root.join(name),
                directory: false,
                size: 0,
            })
            .collect();
        let cancel = AtomicBool::new(false);
        let without = search(&entries, "LEASE CLAUSE 4827", &cancel, false, false).unwrap();
        assert!(without.results.is_empty());
        let with = search(&entries, "LEASE CLAUSE 4827", &cancel, false, true).unwrap();
        assert!(with.warnings.is_empty(), "{:?}", with.warnings);
        assert_eq!(with.results.len(), 2);
        assert_eq!(with.locations[&root.join("scanned.pdf")].page, Some(2));
        assert!(with.locations[&root.join("ocr.png")]
            .snippet
            .contains("4827"));
    }
    #[test]
    fn exact_search_skips_broken_documents_and_supports_unicode_and_cancellation() {
        let root = tempfile::tempdir().unwrap();
        let good = root.path().join("good.txt");
        std::fs::write(&good, "Lease clause: CAFÉ 世界").unwrap();
        let entries = vec![
            Entry {
                path: root.path().join("missing.pdf"),
                directory: false,
                size: 0,
            },
            Entry {
                path: good.clone(),
                directory: false,
                size: 0,
            },
        ];
        let cancel = AtomicBool::new(false);
        let output = search(&entries, "café 世界", &cancel, false, false).unwrap();
        assert_eq!(output.results[0].0, good);
        assert_eq!(output.warnings.len(), 1);
        assert!(search(&entries, "no match", &cancel, false, false)
            .unwrap()
            .results
            .is_empty());
        assert!(search(&entries, " ", &cancel, false, false).is_err());
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            search(&entries, "lease", &cancel, false, false)
                .err()
                .unwrap(),
            "Search cancelled"
        );
    }
    #[test]
    fn source_search_requires_explicit_opt_in() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("main.rs");
        std::fs::write(&path, "fn search_anchor() {}").unwrap();
        let entries = vec![Entry {
            path,
            directory: false,
            size: 0,
        }];
        let cancel = AtomicBool::new(false);
        assert!(search(&entries, "search_anchor", &cancel, false, false)
            .unwrap()
            .results
            .is_empty());
        assert_eq!(
            search(&entries, "search_anchor", &cancel, true, false)
                .unwrap()
                .results
                .len(),
            1
        );
        assert!(!documents::source_file(std::path::Path::new(".env")));
    }
}
