//! Telemetry removed. Inert lifecycle hooks preserve game startup/save call sites.
//! Local crash files and explicit benchmark reports are handled by their own modules.
use horizon_telemetry::payload::Settings;
use std::path::Path;

pub fn mark_start() {}
pub fn mark_ui_ready() {}
pub fn mark_core_ready() {}
pub fn mark_rest_ready() {}
pub fn note_display(_window: &winit::window::Window) {}

/// Local display information for the explicitly requested game-control response.
pub fn monitor(window: &winit::window::Window) -> Option<winit::monitor::MonitorHandle> {
    let only = || {
        let mut all = window.available_monitors();
        all.next().filter(|_| all.next().is_none())
    };
    window
        .current_monitor()
        .or_else(|| window.primary_monitor())
        .or_else(only)
}

#[derive(Default)]
pub struct Session {
    _private: (),
}
impl Session {
    pub fn title_shown(&mut self) {}
    pub fn world_opening(&mut self) {}
    pub fn cancel_world_opening(&mut self) {}
    pub fn frame(&mut self, _ms: f32) {}
    pub fn tick(&mut self, _ms: f32) {}
    pub fn finish(&mut self, _home: &Path, _settings: Settings) {}
}
