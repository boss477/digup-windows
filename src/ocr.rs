use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

fn tesseract() -> PathBuf {
    if let Some(path) = std::env::var_os("DIGUP_TESSERACT") {
        return path.into();
    }
    let installed = PathBuf::from(
        std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into()),
    )
    .join("Tesseract-OCR/tesseract.exe");
    if installed.is_file() {
        installed
    } else {
        PathBuf::from("tesseract")
    }
}
pub fn image(path: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let output = crate::process::output(
        Command::new(tesseract())
            .arg(path)
            .arg("stdout")
            .args(["-l", "eng", "--psm", "3"]),
        cancel,
    )?;
    if !output.status.success() {
        return Err(format!(
            "Tesseract: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}
pub fn pdf_page(path: &Path, page: usize, cancel: &AtomicBool) -> Result<String, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let prefix = temp.path().join("page");
    let renderer = std::env::var_os("DIGUP_PDFTOPPM").unwrap_or_else(|| "pdftoppm".into());
    let output = crate::process::output(
        Command::new(renderer)
            .args([
                "-f",
                &page.to_string(),
                "-l",
                &page.to_string(),
                "-r",
                "150",
                "-scale-to",
                "2400",
                "-singlefile",
                "-png",
            ])
            .arg(path)
            .arg(&prefix),
        cancel,
    )?;
    if !output.status.success() {
        return Err(format!(
            "PDF renderer: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    image(&prefix.with_extension("png"), cancel)
}
