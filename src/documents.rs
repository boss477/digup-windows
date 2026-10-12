use std::{io::Read, path::Path};

const MAX_DOCUMENT: u64 = 50_000_000;
pub struct Section {
    pub page: Option<usize>,
    pub text: String,
}
pub fn sections(
    path: &Path,
    ocr: bool,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<Section>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT {
        return Err("Document exceeds the 50 MB extraction limit".into());
    }
    if extension(path) == "pdf" {
        let document = lopdf::Document::load(path).map_err(|e| e.to_string())?;
        let pages = document.get_pages();
        if pages.is_empty() {
            return Err("PDF has no readable pages".into());
        }
        if pages.len() > 1000 {
            return Err("PDF exceeds the 1,000 page limit".into());
        }
        pages
            .into_keys()
            .map(|page| {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("Search cancelled".into());
                }
                let text = document.extract_text(&[page]).map_err(|e| e.to_string())?;
                let text = if ocr && text.trim().is_empty() {
                    crate::ocr::pdf_page(path, page as usize, cancel)?
                } else {
                    text
                };
                Ok(Section {
                    page: Some(page as usize),
                    text,
                })
            })
            .collect()
    } else if crate::media::is_image(path) && ocr {
        Ok(vec![Section {
            page: None,
            text: crate::ocr::image(path, cancel)?,
        }])
    } else {
        Ok(vec![Section {
            page: None,
            text: extract(path)?,
        }])
    }
}
pub fn supported(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "txt" | "md" | "rst" | "pdf" | "docx" | "odt"
    )
}
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}
pub fn source_file(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "go"
            | "cs"
            | "sql"
            | "css"
            | "html"
            | "xml"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "ps1"
            | "sh"
    )
}
pub fn extract(path: &Path) -> Result<String, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT {
        return Err("Document exceeds the 50 MB extraction limit".into());
    }
    match extension(path).as_str() {
        "pdf" => {
            let document = lopdf::Document::load(path).map_err(|e| e.to_string())?;
            let pages: Vec<_> = document.get_pages().into_keys().collect();
            if pages.len() > 1000 {
                return Err("PDF exceeds the 1,000 page limit".into());
            }
            document.extract_text(&pages).map_err(|e| e.to_string())
        }
        "docx" | "odt" => {
            let docx = extension(path) == "docx";
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let mut part = archive
                .by_name(if docx {
                    "word/document.xml"
                } else {
                    "content.xml"
                })
                .map_err(|e| e.to_string())?;
            if part.size() > MAX_DOCUMENT {
                return Err("Expanded document exceeds 50 MB".into());
            }
            let mut xml = String::new();
            part.read_to_string(&mut xml).map_err(|e| e.to_string())?;
            xml_text(&xml)
        }
        "txt" | "md" | "rst" => std::fs::read_to_string(path).map_err(|e| e.to_string()),
        _ if source_file(path) => std::fs::read_to_string(path).map_err(|e| e.to_string()),
        _ => Err("Unsupported document format".into()),
    }
}
fn xml_text(xml: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let mut output = String::new();
    // Paragraph boundaries separate words while adjacent formatting runs stay joined.
    for paragraph in doc
        .descendants()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "p" | "h"))
    {
        for node in paragraph.descendants().filter(|n| n.is_text()) {
            output.push_str(node.text().unwrap_or_default());
        }
        output.push('\n');
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn office_archives_preserve_entities_runs_and_paragraphs() {
        let root = tempfile::tempdir().unwrap();
        for (ext,part,xml) in [
            ("docx", "word/document.xml", "<w:document xmlns:w='urn:word'><w:p><w:r><w:t>Hello &amp; </w:t></w:r><w:r><w:t>世界</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t></w:r></w:p></w:document>"),
            ("odt", "content.xml", "<document><p>Hello &amp; 世界</p><p>Second</p></document>")
        ] {
            let path = root.path().join(format!("fixture.{ext}"));
            let mut archive = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            archive.start_file(part, zip::write::SimpleFileOptions::default()).unwrap();
            archive.write_all(xml.as_bytes()).unwrap(); archive.finish().unwrap();
            assert_eq!(extract(&path).unwrap(), "Hello & 世界\nSecond\n");
        }
    }
    #[test]
    fn pdf_fixture_extracts_visible_text() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("fixture.pdf");
        let stream = "BT /F1 12 Tf 72 720 Td (Searchable lease clause) Tj ET";
        let objects = ["<< /Type /Catalog /Pages 2 0 R >>".to_owned(), "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(), "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".into(), "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(), format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len())];
        let mut pdf = "%PDF-1.4\n".to_owned();
        let mut offsets = vec![];
        for (i, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
        }
        let xref = pdf.len();
        pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
        ));
        std::fs::write(&path, pdf).unwrap();
        assert!(extract(&path).unwrap().contains("Searchable lease clause"));
    }
}
