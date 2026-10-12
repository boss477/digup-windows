use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;

fn command(name: &str) -> Command {
    let mut command = Command::new(name);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
#[cfg(test)]
pub fn duration(path: &Path) -> Result<f64, String> {
    duration_cancel(path, &AtomicBool::new(false))
}
pub fn duration_cancel(path: &Path, cancel: &AtomicBool) -> Result<f64, String> {
    let output = crate::process::output(
        command("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path),
        cancel,
    )?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let seconds: f64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| "Cannot determine media duration")?;
    if !seconds.is_finite() || seconds <= 0. {
        return Err("Invalid media duration".into());
    }
    Ok(seconds)
}
#[cfg(test)]
pub fn has_audio(path: &Path) -> Result<bool, String> {
    has_audio_cancel(path, &AtomicBool::new(false))
}
pub fn has_audio_cancel(path: &Path, cancel: &AtomicBool) -> Result<bool, String> {
    let output = crate::process::output(
        command("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=index",
                "-of",
                "csv=p=0",
            ])
            .arg(path),
        cancel,
    )?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
#[cfg(test)]
pub fn audio_input(path: &Path, start: f64) -> Result<serde_json::Value, String> {
    audio_input_cancel(path, start, &AtomicBool::new(false))
}
pub fn audio_input_cancel(
    path: &Path,
    start: f64,
    cancel: &AtomicBool,
) -> Result<serde_json::Value, String> {
    let output = crate::process::output(
        command("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-ss",
                &start.to_string(),
                "-i",
            ])
            .arg(path)
            .args([
                "-t", "30", "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", "pipe:1",
            ]),
        cancel,
    )?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(
        serde_json::json!([{"content":[{"type":"input_audio","input_audio":{"data":STANDARD.encode(output.stdout),"format":"wav"}}]}]),
    )
}
#[cfg(test)]
pub fn video_frame(path: &Path, start: f64) -> Result<serde_json::Value, String> {
    video_frame_cancel(path, start, &AtomicBool::new(false))
}
pub fn video_frame_cancel(
    path: &Path,
    start: f64,
    cancel: &AtomicBool,
) -> Result<serde_json::Value, String> {
    let output = crate::process::output(
        command("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-ss",
                &start.to_string(),
                "-i",
            ])
            .arg(path)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale=640:-2",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "pipe:1",
            ]),
        cancel,
    )?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(format!(
            "Cannot read video frame: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(
        serde_json::json!([{"content":[{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{}", STANDARD.encode(output.stdout))}}]}]),
    )
}
pub fn is_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "wav" | "mp3" | "flac" | "m4a" | "ogg" | "aac"
    )
}
pub fn is_video(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "mkv" | "mov" | "webm" | "avi"
    )
}

pub fn image_input(path: &Path) -> Result<serde_json::Value, String> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => return Err("Supported query images: PNG, JPEG, WebP, BMP".into()),
    };
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 20_000_000 {
        return Err("Image exceeds the 20 MB input limit".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(
        serde_json::json!([{"content":[{"type":"image_url","image_url":{"url":format!("data:{mime};base64,{}", STANDARD.encode(bytes))}}]}]),
    )
}
pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp"
    )
}
