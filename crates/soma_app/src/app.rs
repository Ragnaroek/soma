use std::sync::{Arc, RwLock};

use libsoma::dmg;

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

pub struct SomaApp {
    fb: Arc<RwLock<FrameBuffer>>,
}

impl SomaApp {
    pub fn new(cc: &eframe::CreationContext<'_>, fb: Arc<RwLock<FrameBuffer>>) -> SomaApp {
        SomaApp { fb }
    }
}

impl eframe::App for SomaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let fb = self.fb.read().unwrap();

        if fb.needs_update {
            let image =
                egui::ColorImage::from_rgb([dmg::RESOLUTION_X, dmg::RESOLUTION_Y], &fb.buffer);
            let texture_handle = ui.load_texture("frame", image, egui::TextureOptions::default());
            ui.image(&texture_handle);

            ui.request_repaint();
        }
    }
}
