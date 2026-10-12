use eframe::egui::{Context, ViewportCommand};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct Desktop {
    _tray: TrayIcon,
    _hotkeys: GlobalHotKeyManager,
    quit: Arc<AtomicBool>,
    pub hotkey_warning: Option<String>,
}
pub fn hide_on_close(tray_available: bool, quitting: bool, close_requested: bool) -> bool {
    tray_available && !quitting && close_requested
}
fn show(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    ctx.request_repaint();
}
impl Desktop {
    pub fn new(ctx: Context) -> Result<Self, String> {
        let hotkeys = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let hotkey_warning = hotkeys
            .register(HotKey::new(
                Some(Modifiers::CONTROL | Modifiers::SHIFT),
                Code::Space,
            ))
            .err()
            .map(|e| {
                format!("Ctrl+Shift+Space unavailable: {e}. Use the tray menu to show DigUp.")
            });
        let menu = Menu::new();
        let open = MenuItem::new("Show DigUp", true, None);
        let exit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&open, &exit])
            .map_err(|e| e.to_string())?;
        let icon = Icon::from_rgba([40, 180, 170, 255].repeat(32 * 32), 32, 32)
            .map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("DigUp · Ctrl+Shift+Space")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())?;
        let quit = Arc::new(AtomicBool::new(false));
        let quitting = quit.clone();
        let menu_ctx = ctx.clone();
        let open_id = open.id().clone();
        let exit_id = exit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == open_id {
                show(&menu_ctx);
            }
            if event.id == exit_id {
                quitting.store(true, Ordering::Relaxed);
                menu_ctx.send_viewport_cmd(ViewportCommand::Close);
                menu_ctx.request_repaint();
            }
        }));
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                show(&ctx);
            }
        }));
        Ok(Self {
            _tray: tray,
            _hotkeys: hotkeys,
            quit,
            hotkey_warning,
        })
    }
    pub fn quitting(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_hides_only_with_a_live_tray_and_never_blocks_quit() {
        assert!(hide_on_close(true, false, true));
        assert!(!hide_on_close(false, false, true));
        assert!(!hide_on_close(true, true, true));
        assert!(!hide_on_close(true, false, false));
    }
}
