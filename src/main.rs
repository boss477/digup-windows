#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
mod cache;
mod desktop;
mod documents;
mod exact;
mod health;
mod location;
mod media;
mod ocr;
mod process;
mod ranking;
mod watch;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    path: PathBuf,
    directory: bool,
    size: u64,
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Saved {
    entries: Vec<Entry>,
    url: String,
    model: String,
    provider: usize,
    include_images: bool,
    include_av: bool,
    include_code: bool,
    include_ocr: bool,
    root: Option<PathBuf>,
}
fn state_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "DigUpWindows")
        .map(|d| d.data_local_dir().join("state.json"))
}
struct SearchOutput {
    results: Vec<(PathBuf, f64)>,
    warnings: Vec<String>,
    locations: std::collections::HashMap<PathBuf, location::Match>,
}
enum Event {
    Indexed(u64, Vec<Entry>, bool),
    WatchError(u64, String),
    Ranked(SearchOutput),
    Error(String),
}
struct App {
    entries: Vec<Entry>,
    query: String,
    regex: bool,
    folders: bool,
    url: String,
    model: String,
    provider: usize,
    busy: bool,
    searching: bool,
    status: String,
    results: Vec<(PathBuf, f64)>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    wildcard: bool,
    sort: usize,
    image_query: Option<PathBuf>,
    include_images: bool,
    include_av: bool,
    include_code: bool,
    include_ocr: bool,
    locations: std::collections::HashMap<PathBuf, location::Match>,
    warnings: Vec<String>,
    root: Option<PathBuf>,
    watcher: Option<watch::Watch>,
    generation: u64,
    watch_error: Option<String>,
    desktop: Option<desktop::Desktop>,
    desktop_error: Option<String>,
    cancel: Arc<AtomicBool>,
}
impl Default for App {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let saved: Saved = state_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let root = saved.root.clone();
        let watcher = root
            .as_ref()
            .map(|r| watch::start(r.clone(), 0, tx.clone()));
        Self {
            entries: saved.entries,
            query: String::new(),
            regex: false,
            folders: false,
            url: if saved.url.is_empty() {
                "http://localhost:1234/v1".into()
            } else {
                saved.url
            },
            model: saved.model,
            provider: saved.provider,
            busy: root.is_some(),
            searching: false,
            status: "Choose a folder to refresh the saved filename index.".into(),
            results: vec![],
            tx,
            rx,
            wildcard: false,
            sort: 0,
            image_query: None,
            include_images: saved.include_images,
            include_av: saved.include_av,
            include_code: saved.include_code,
            include_ocr: saved.include_ocr,
            locations: Default::default(),
            warnings: vec![],
            root,
            watcher,
            generation: 0,
            watch_error: None,
            desktop: None,
            desktop_error: None,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
fn embedding(
    client: &reqwest::blocking::Client,
    url: &str,
    model: &str,
    input: &str,
) -> Result<Vec<f64>, String> {
    embedding_input(client, url, model, serde_json::json!(input))
}
fn embedding_input(
    client: &reqwest::blocking::Client,
    url: &str,
    model: &str,
    input: serde_json::Value,
) -> Result<Vec<f64>, String> {
    let body: serde_json::Value = client
        .post(format!("{}/embeddings", url.trim_end_matches('/')))
        .json(&serde_json::json!({"model":model,"input":input}))
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    let values = body["data"][0]["embedding"]
        .as_array()
        .ok_or("Server returned no embedding")?;
    let vector: Vec<f64> = values
        .iter()
        .map(|v| {
            v.as_f64()
                .filter(|n| n.is_finite())
                .ok_or("Invalid embedding value".to_string())
        })
        .collect::<Result<_, _>>()?;
    if vector.is_empty() {
        return Err("Empty embedding".into());
    }
    Ok(vector)
}
fn cosine(a: &[f64], b: &[f64]) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err("Embedding dimensions changed; use the same model".into());
    }
    let norm =
        a.iter().map(|x| x * x).sum::<f64>().sqrt() * b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm == 0.0 {
        return Err("Zero embedding".into());
    }
    Ok(a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>() / norm)
}
impl eframe::App for App {
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(path) = state_path() {
            let saved = Saved {
                entries: self.entries.clone(),
                url: self.url.clone(),
                model: self.model.clone(),
                provider: self.provider,
                include_images: self.include_images,
                include_av: self.include_av,
                include_code: self.include_code,
                include_ocr: self.include_ocr,
                root: self.root.clone(),
            };
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(bytes) = serde_json::to_vec(&saved) {
                let _ = std::fs::write(path, bytes);
            }
        }
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if desktop::hide_on_close(
            self.desktop.is_some(),
            self.desktop.as_ref().is_some_and(|d| d.quitting()),
            ctx.input(|i| i.viewport().close_requested()),
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Indexed(generation, entries, initial) => {
                    if generation != self.generation {
                        continue;
                    }
                    if initial {
                        self.busy = false;
                        self.results.clear();
                    }
                    if !self.busy {
                        self.status = format!("{} filenames indexed", entries.len());
                    }
                    self.entries = entries;
                    self.results
                        .retain(|(path, _)| self.entries.iter().any(|entry| &entry.path == path));
                }
                Event::WatchError(generation, error) => {
                    if generation == self.generation {
                        self.watch_error = Some(error);
                    }
                }
                Event::Ranked(SearchOutput {
                    results,
                    warnings,
                    locations,
                }) => {
                    self.busy = false;
                    self.searching = false;
                    self.status = format!(
                        "{} content matches; {} files skipped",
                        results.len(),
                        warnings.len()
                    );
                    self.results = results;
                    self.warnings = warnings;
                    self.locations = locations;
                }
                Event::Error(error) => {
                    self.busy = false;
                    self.searching = false;
                    self.status = error;
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DigUp · Windows");
            if self.desktop.is_some() {
                ui.label(
                    "Ctrl+Shift+Space to show · Close hides to tray · Tray menu → Quit to exit",
                );
            }
            if let Some(warning) = self
                .desktop
                .as_ref()
                .and_then(|d| d.hotkey_warning.as_ref())
            {
                ui.colored_label(egui::Color32::YELLOW, warning);
            }
            if let Some(error) = &self.desktop_error {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("Desktop integration unavailable: {error}"),
                );
            }
            ui.label("Fast filenames + semantic file contents");
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Choose folder"))
                    .clicked()
                {
                    if let Some(root) = rfd::FileDialog::new().pick_folder() {
                        self.busy = true;
                        self.status = "Scanning filenames…".into();
                        self.generation += 1;
                        self.watch_error = None;
                        self.root = Some(root.clone());
                        self.watcher = Some(watch::start(root, self.generation, self.tx.clone()));
                    }
                }
                ui.checkbox(&mut self.regex, "Regex");
                ui.checkbox(&mut self.folders, "Folders only");
                ui.checkbox(&mut self.wildcard, "Wildcard");
                egui::ComboBox::from_id_salt("sort")
                    .selected_text(if self.sort == 0 { "Path" } else { "Size" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort, 0, "Path");
                        ui.selectable_value(&mut self.sort, 1, "Size");
                    });
            });
            ui.text_edit_singleline(&mut self.query);
            if self.searching && ui.button("Cancel content search").clicked() {
                self.cancel.store(true, Ordering::Relaxed);
                self.status = "Cancellation requested; waiting for the current operation…".into();
            }
            if ui
                .add_enabled(
                    !self.busy && !self.query.trim().is_empty(),
                    egui::Button::new("Find exact text in documents"),
                )
                .clicked()
            {
                self.busy = true;
                self.cancel.store(false, Ordering::Relaxed);
                self.results.clear();
                self.warnings.clear();
                self.status = "Searching document text locally…".into();
                self.searching = true;
                let entries = self.entries.clone();
                let query = self.query.to_lowercase();
                let cancel = self.cancel.clone();
                let tx = self.tx.clone();
                let include_code = self.include_code;
                let include_ocr = self.include_ocr;
                std::thread::spawn(move || {
                    let _ = tx.send(match exact::search(&entries, &query, &cancel, include_code, include_ocr) {
                        Ok(output) => Event::Ranked(output),
                        Err(error) => Event::Error(error),
                    });
                });
            }
            ui.horizontal(|ui| {
                if ui.button("Query with image…").clicked() {
                    self.image_query = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_file();
                }
                if let Some(path) = &self.image_query {
                    ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                }
                if ui.button("Clear image").clicked() {
                    self.image_query = None;
                }
                ui.checkbox(&mut self.include_images, "Search image contents");
                ui.checkbox(&mut self.include_av, "Search audio/video contents");
                ui.checkbox(&mut self.include_code, "Include source/config files");
                ui.checkbox(&mut self.include_ocr, "OCR images/scanned PDF pages");
            });
            if self.include_code {
                ui.label("Source files may contain secrets. Exact search stays local; semantic search sends content to your configured endpoint. Gitignore rules are not applied.");
            }
            ui.collapsing("Inference settings", |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_value(&mut self.provider, 0, "LM Studio")
                        .clicked()
                    {
                        self.url = "http://localhost:1234/v1".into();
                    }
                    if ui
                        .selectable_value(&mut self.provider, 1, "llama.cpp server")
                        .clicked()
                    {
                        self.url = "http://localhost:8080/v1".into();
                    }
                });
                ui.label("API base URL (any port)");
                ui.text_edit_singleline(&mut self.url);
                ui.label("Exact embedding model ID");
                ui.text_edit_singleline(&mut self.model);
            });
            if ui
                .add_enabled(
                    !self.busy
                        && (!self.query.trim().is_empty() || self.image_query.is_some())
                        && !self.model.trim().is_empty(),
                    egui::Button::new("Search contents semantically"),
                )
                .clicked()
            {
                self.busy = true;
                self.cancel.store(false, Ordering::Relaxed);
                let cancel = self.cancel.clone();
                self.results.clear();
                self.warnings.clear();
                self.status = "Searching contents; the first pass may take a while…".into();
                self.searching = true;
                let image_query = self.image_query.clone();
                let include_images = self.include_images;
                let include_av = self.include_av;
                let include_code = self.include_code;
                let include_ocr = self.include_ocr;
                let (entries, query, url, model, tx) = (
                    self.entries.clone(),
                    self.query.clone(),
                    self.url.clone(),
                    self.model.clone(),
                    self.tx.clone(),
                );
                std::thread::spawn(move || {
                    let run = || -> Result<SearchOutput, String> {
                        let client = reqwest::blocking::Client::builder()
                            .timeout(Duration::from_secs(120))
                            .build()
                            .map_err(|e| e.to_string())?;
                        // EmbeddingGemma 2 uses asymmetric query/document task instructions.
                        let gemma = model.to_lowercase().contains("embeddinggemma");
                        let query_input = if gemma {
                            format!("task: search result | query: {query}")
                        } else {
                            query.clone()
                        };
                        let q = if let Some(path) = &image_query {
                            embedding_input(&client, &url, &model, media::image_input(path)?)?
                        } else {
                            embedding(&client, &url, &model, &query_input)?
                        };
                        let mut results = vec![];
                        let mut warnings = vec![];
                        let mut locations = std::collections::HashMap::new();
                        let db_path = state_path()
                            .ok_or("Cannot locate local data directory")?
                            .with_file_name("embeddings.sqlite");
                        std::fs::create_dir_all(db_path.parent().ok_or("Missing data directory")?)
                            .map_err(|e| e.to_string())?;
                        let cache = cache::Cache::open(&db_path)?;
                        for entry in entries.iter().filter(|e| !e.directory) {
                            if cancel.load(Ordering::Relaxed) {
                                return Err("Search cancelled".into());
                            }
                            let file_result = (|| -> Result<(), String> {
                                if include_av
                                    && (media::is_audio(&entry.path)
                                        || media::is_video(&entry.path))
                                {
                                    let duration = media::duration_cancel(&entry.path, &cancel)?;
                                    let mut best = -1.0_f64;
                                    let video = media::is_video(&entry.path);
                                    let interval = if video { 5. } else { 25. };
                                    let mut start = 0.;
                                    while start < duration {
                                        if cancel.load(Ordering::Relaxed) {
                                            return Err("Search cancelled".into());
                                        }
                                        let input = if video {
                                            media::video_frame_cancel(&entry.path, start, &cancel)?
                                        } else {
                                            media::audio_input_cancel(&entry.path, start, &cancel)?
                                        };
                                        let key =
                                            cache::Cache::key(&url, &model, &input.to_string());
                                        let vector = match cache.get(&key)? {
                                            Some(v) => v,
                                            None => {
                                                let v =
                                                    embedding_input(&client, &url, &model, input)?;
                                                cache.put(&key, &v)?;
                                                v
                                            }
                                        };
                                        let score = cosine(&q, &vector)?;
                                        if score > best { best = score; locations.insert(entry.path.clone(), location::Match { seconds: Some(start), snippet: if video { "Video frame".into() } else { "Audio window".into() }, ..Default::default() }); }
                                        start += interval;
                                    }
                                    if video && media::has_audio_cancel(&entry.path, &cancel)? {
                                        let mut start = 0.;
                                        while start < duration {
                                            if cancel.load(Ordering::Relaxed) {
                                                return Err("Search cancelled".into());
                                            }
                                            let input = media::audio_input_cancel(&entry.path, start, &cancel)?;
                                            let key =
                                                cache::Cache::key(&url, &model, &input.to_string());
                                            let vector = match cache.get(&key)? {
                                                Some(v) => v,
                                                None => {
                                                    let v = embedding_input(
                                                        &client, &url, &model, input,
                                                    )?;
                                                    cache.put(&key, &v)?;
                                                    v
                                                }
                                            };
                                            let score = cosine(&q, &vector)?;
                                            if score > best { best = score; locations.insert(entry.path.clone(), location::Match { seconds: Some(start), snippet: "Video soundtrack".into(), ..Default::default() }); }
                                            start += 25.;
                                        }
                                    }
                                    results.push((entry.path.clone(), best));
                                    return Ok(());
                                }
                                if include_images && media::is_image(&entry.path) {
                                    let input = media::image_input(&entry.path)?;
                                    let key = cache::Cache::key(&url, &model, &input.to_string());
                                    let vector = match cache.get(&key)? {
                                        Some(v) => v,
                                        None => {
                                            let v = embedding_input(&client, &url, &model, input)?;
                                            cache.put(&key, &v)?;
                                            v
                                        }
                                    };
                                    results.push((entry.path.clone(), cosine(&q, &vector)?));
                                    if !include_ocr { return Ok(()); }
                                }
                                if !documents::supported(&entry.path) && !(include_code && documents::source_file(&entry.path)) && !(include_ocr && media::is_image(&entry.path)) {
                                    return Ok(());
                                }
                                let sections = match documents::sections(&entry.path, include_ocr, &cancel) {
                                    Ok(sections) => sections,
                                    Err(error) => {
                                        warnings.push(format!("{}: {error}", entry.path.display()));
                                        return Ok(());
                                    }
                                };
                                let existing = results.iter().find(|(path, _)| path == &entry.path).map(|(_,score)| *score);
                                let mut best = existing.unwrap_or(-1.0_f64);
                                let mut found = existing.is_some();
                                for section in sections {
                                for chunk in cache::chunks(&section.text) {
                                    if cancel.load(Ordering::Relaxed) {
                                        return Err("Search cancelled".into());
                                    }
                                    let input = if gemma {
                                        format!(
                                            "title: {} | text: {chunk}",
                                            entry
                                                .path
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                        )
                                    } else {
                                        chunk.clone()
                                    };
                                    let key = cache::Cache::key(&url, &model, &input);
                                    let vector = match cache.get(&key)? {
                                        Some(vector) => vector,
                                        None => {
                                            let vector = embedding(&client, &url, &model, &input)?;
                                            cache.put(&key, &vector)?;
                                            vector
                                        }
                                    };
                                    let score = cosine(&q, &vector)?;
                                    if !found || score > best { best = score; locations.insert(entry.path.clone(), location::Match { page: section.page, seconds: None, snippet: chunk.chars().take(300).collect() }); }
                                    found = true;
                                }
                                }
                                if found {
                                    results.retain(|(path,_)| path != &entry.path);
                                    results.push((entry.path.clone(), best));
                                }
                                Ok(())
                            })();
                            if cancel.load(Ordering::Relaxed) {
                                return Err("Search cancelled".into());
                            }
                            if let Err(error) = file_result {
                                warnings.push(format!("{}: {error}", entry.path.display()));
                            }
                        }
                        ranking::calibrate(&mut results);
                        results.sort_by(|a, b| b.1.total_cmp(&a.1));
                        results.truncate(100);
                        Ok(SearchOutput { results, warnings, locations })
                    };
                    let _ = tx.send(match run() {
                        Ok(output) => Event::Ranked(output),
                        Err(e) => Event::Error(e),
                    });
                });
            }
            ui.label(&self.status);
            if let Some(error) = &self.watch_error {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("Folder watch unavailable: {error}"),
                );
            }
            if !self.warnings.is_empty() {
                ui.collapsing("Skipped files", |ui| {
                    for warning in &self.warnings {
                        ui.label(warning);
                    }
                });
            }
            let pattern = if self.regex {
                Some(regex::Regex::new(&self.query))
            } else if self.wildcard {
                // Escape user literals before introducing wildcard operators.
                Some(regex::Regex::new(&format!(
                    "(?i){}",
                    regex::escape(&self.query)
                        .replace("\\*", ".*")
                        .replace("\\?", ".")
                )))
            } else {
                None
            };
            if let Some(Err(error)) = &pattern {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
            let query = self.query.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (path, score) in &self.results {
                    ui.horizontal(|ui| {
                        if ui.small_button("Open").clicked() {
                            let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
                        }
                        if ui.small_button("Show").clicked() {
                            let _ = std::process::Command::new("explorer.exe")
                                .arg(format!("/select,{}", path.display()))
                                .spawn();
                        }
                        ui.label(format!("{score:.3}  {}", path.display()));
                        if let Some(location) = self.locations.get(path) {
                            ui.label(location.label()).on_hover_text(&location.snippet);
                            if ui.small_button("Open match").clicked() {
                                if let Err(error) = location::open(path, location) { self.status = error; }
                            }
                        }
                    });
                }
                ui.separator();
                let mut matches: Vec<_> = self
                    .entries
                    .iter()
                    .filter(|e| {
                        if self.folders && !e.directory {
                            return false;
                        }
                        let path = e.path.to_string_lossy();
                        match &pattern {
                            Some(Ok(re)) => re.is_match(&path),
                            Some(Err(_)) => false,
                            None => path.to_lowercase().contains(&query),
                        }
                    })
                    .collect();
                if self.sort == 0 {
                    matches.sort_by(|a, b| a.path.cmp(&b.path));
                } else {
                    matches.sort_by_key(|e| std::cmp::Reverse(e.size));
                }
                for entry in matches.into_iter().take(500) {
                    ui.horizontal(|ui| {
                        if ui.small_button("Show").clicked() {
                            let _ = std::process::Command::new("explorer.exe")
                                .arg(format!("/select,{}", entry.path.display()))
                                .spawn();
                        }
                        ui.label(entry.path.to_string_lossy());
                    });
                }
            });
        });
        ctx.request_repaint_after(Duration::from_millis(if self.busy { 100 } else { 500 }));
    }
}
fn main() -> eframe::Result {
    if std::env::args().any(|argument| argument == "--self-check") {
        match health::run() {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("Self-check failed: {error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    eframe::run_native(
        "DigUp Windows",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            let mut app = App::default();
            match desktop::Desktop::new(cc.egui_ctx.clone()) {
                Ok(desktop) => app.desktop = Some(desktop),
                Err(error) => app.desktop_error = Some(error),
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires local multimodal llama.cpp server and generated image fixture"]
    fn live_multimodal_embeddings() {
        let url = std::env::var("DIGUP_LIVE_URL").expect("Set DIGUP_LIVE_URL");
        let fixture =
            PathBuf::from(std::env::var("DIGUP_IMAGE_FIXTURE").expect("Set DIGUP_IMAGE_FIXTURE"));
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();
        let image = embedding_input(
            &client,
            &url,
            "embeddinggemma-2",
            media::image_input(&fixture).unwrap(),
        )
        .unwrap();
        let red = embedding(
            &client,
            &url,
            "embeddinggemma-2",
            "task: search result | query: a solid red square",
        )
        .unwrap();
        let blue = embedding(
            &client,
            &url,
            "embeddinggemma-2",
            "task: search result | query: a solid blue square",
        )
        .unwrap();
        assert_eq!(image.len(), 768);
        assert!(cosine(&image, &red).unwrap() > cosine(&image, &blue).unwrap());
        let root = fixture.parent().unwrap();
        let audio_path = root.join("tone-fixture.wav");
        assert!(media::has_audio(&audio_path).unwrap());
        assert!(!media::has_audio(&root.join("video-fixture.mp4")).unwrap());
        assert!((media::duration(&audio_path).unwrap() - 1.).abs() < 0.1);
        let audio = embedding_input(
            &client,
            &url,
            "embeddinggemma-2",
            media::audio_input(&audio_path, 0.).unwrap(),
        )
        .unwrap();
        assert_eq!(audio.len(), 768);
        let video = embedding_input(
            &client,
            &url,
            "embeddinggemma-2",
            media::video_frame(&root.join("video-fixture.mp4"), 0.).unwrap(),
        )
        .unwrap();
        assert_eq!(video.len(), 768);
    }
    #[test]
    fn cosine_ranks_related_vectors_and_rejects_invalid_shapes() {
        assert!((cosine(&[1., 0.], &[1., 0.]).unwrap() - 1.).abs() < 1e-10);
        assert!(cosine(&[1., 0.], &[0., 1.]).unwrap().abs() < 1e-10);
        assert!(cosine(&[1.], &[1., 0.]).is_err());
        assert!(cosine(&[0., 0.], &[1., 0.]).is_err());
    }
    #[test]
    fn saved_state_roundtrips_unicode_paths() {
        let state = Saved {
            entries: vec![Entry {
                path: PathBuf::from("C:/测试/my file.txt"),
                directory: false,
                size: 42,
            }],
            url: "http://localhost:5678/v1".into(),
            model: "embedding".into(),
            provider: 1,
            include_images: true,
            include_av: false,
            include_code: false,
            include_ocr: false,
            root: None,
        };
        let loaded: Saved = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        assert_eq!(loaded.entries[0].path, state.entries[0].path);
        assert_eq!(loaded.url, state.url);
    }
}
