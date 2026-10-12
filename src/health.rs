use std::sync::atomic::AtomicBool;

pub fn run() -> Result<serde_json::Value, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = temp.path().join("smoke.txt");
    std::fs::write(&path, "DigUp portable self check 世界").map_err(|e| e.to_string())?;
    let entries = vec![crate::Entry {
        path,
        directory: false,
        size: 0,
    }];
    let output = crate::exact::search(
        &entries,
        "self check 世界",
        &AtomicBool::new(false),
        false,
        false,
    )?;
    if output.results.len() != 1 {
        return Err("Local document search self-check failed".into());
    }
    let cache = crate::cache::Cache::open(&temp.path().join("cache.sqlite"))?;
    cache.put("smoke", &[1., 0.])?;
    if cache.get("smoke")? != Some(vec![1., 0.]) {
        return Err("SQLite self-check failed".into());
    }
    if (crate::cosine(&[1., 0.], &[1., 0.])? - 1.).abs() > 1e-9 {
        return Err("Vector self-check failed".into());
    }
    Ok(
        serde_json::json!({"status":"ok", "version":env!("CARGO_PKG_VERSION"), "architecture":std::env::consts::ARCH, "checks":["temporary-file IO", "Unicode exact search", "SQLite vector cache", "cosine ranking"], "external_tools":"Optional OCR, media, PDF viewer and inference servers are separate prerequisites; not exercised by this smoke check."}),
    )
}
