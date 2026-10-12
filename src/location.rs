use std::{path::Path, process::Command};

#[derive(Clone, Debug, Default)]
pub struct Match {
    pub page: Option<usize>,
    pub seconds: Option<f64>,
    pub snippet: String,
}
impl Match {
    pub fn label(&self) -> String {
        if let Some(page) = self.page {
            format!("Page {page}")
        } else if let Some(seconds) = self.seconds {
            format!("At {:02}:{:02}", seconds as u64 / 60, seconds as u64 % 60)
        } else {
            "Matching passage".into()
        }
    }
}
pub fn pdf_url(path: &Path, page: usize) -> Result<String, String> {
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let mut url =
        reqwest::Url::from_file_path(canonical).map_err(|_| "Cannot form PDF file URL")?;
    url.set_fragment(Some(&format!("page={page}")));
    Ok(url.to_string())
}
pub fn open(path: &Path, location: &Match) -> Result<(), String> {
    if let Some(page) = location.page {
        // Browser PDF viewers understand #page; default shell PDF handlers may ignore it.
        let url = pdf_url(path, page)?;
        Command::new("explorer.exe")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else if let Some(seconds) = location.seconds {
        let player = std::env::var_os("DIGUP_VLC")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:/Program Files/VideoLAN/VLC/vlc.exe"));
        if !player.is_file() {
            return Err("Timestamp seeking requires VLC. Install VLC or set DIGUP_VLC to vlc.exe. The match time is shown in the result.".into());
        }
        Command::new(player)
            .args(seek_arguments(path, seconds)?)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else {
        Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn seek_arguments(path: &Path, seconds: f64) -> Result<Vec<std::ffi::OsString>, String> {
    if !seconds.is_finite() || seconds < 0. {
        return Err("Invalid match timestamp".into());
    }
    Ok(vec![
        "--no-one-instance".into(),
        format!("--start-time={seconds:.3}").into(),
        path.as_os_str().to_owned(),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_urls_encode_paths_and_timestamps_format() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("test # 世界.pdf");
        std::fs::write(&path, b"fixture").unwrap();
        let url = pdf_url(&path, 3).unwrap();
        assert!(url.ends_with("#page=3"));
        assert!(url.contains("%23"));
        assert_eq!(
            Match {
                seconds: Some(65.),
                ..Default::default()
            }
            .label(),
            "At 01:05"
        );
    }
    #[test]
    fn seeking_keeps_unicode_paths_as_one_argument_and_validates_time() {
        let path = Path::new("C:/视频/my clip.mp4");
        let args = seek_arguments(path, 65.25).unwrap();
        assert_eq!(args[1], "--start-time=65.250");
        assert_eq!(args[2], path.as_os_str());
        assert!(seek_arguments(path, f64::NAN).is_err());
        assert!(seek_arguments(path, -1.).is_err());
    }
}
