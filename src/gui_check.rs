//! Opt-in native rendering smoke test; never registers tray/hotkeys or saves settings.
use crate::{egui, App};
use eframe::glow::HasContext;
use std::path::PathBuf;

struct Check {
    app: App,
    report: PathBuf,
    renderer: String,
    vendor: String,
    requested: bool,
}

impl eframe::App for Check {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.update(ctx, frame);
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let colors: std::collections::HashSet<_> =
                image.pixels.iter().map(|pixel| pixel.to_array()).collect();
            let valid = image.size[0] >= 100 && image.size[1] >= 100 && colors.len() >= 32;
            let report = serde_json::json!({
                "status": if valid { "ok" } else { "failed" },
                "renderer": self.renderer, "vendor": self.vendor,
                "width": image.size[0], "height": image.size[1], "distinct_colors": colors.len(),
                "scope": "native window, actual app UI, rendered framebuffer; no input/tray/hotkey certification"
            });
            std::fs::write(&self.report, serde_json::to_vec_pretty(&report).unwrap())
                .expect("write GUI report");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        ctx.request_repaint();
    }
}

pub fn run(report: PathBuf) -> eframe::Result {
    eframe::run_native(
        "DigUp Windows render check",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            let gl = cc
                .gl
                .as_ref()
                .ok_or("GUI check requires the default OpenGL renderer")?;
            // Read-only GL identity queries on eframe's current context.
            let (renderer, vendor) = unsafe {
                (
                    gl.get_parameter_string(eframe::glow::RENDERER),
                    gl.get_parameter_string(eframe::glow::VENDOR),
                )
            };
            Ok(Box::new(Check {
                app: App::default(),
                report,
                renderer,
                vendor,
                requested: false,
            }))
        }),
    )
}
