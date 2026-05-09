use std::sync::{Arc, RwLock};

use egui::CentralPanel;

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

pub struct SomaApp {
    fb: Arc<RwLock<FrameBuffer>>,
}

impl SomaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, fb: Arc<RwLock<FrameBuffer>>) -> SomaApp {
        SomaApp { fb }
    }
}

const REG_PANEL_WIDTH: f32 = 120.0;
const MARGIN: f32 = 5.0;

impl eframe::App for SomaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                let fb = self.fb.read().unwrap();

                if fb.needs_update {
                    let image = egui::ColorImage::from_rgb(
                        [256, 256], /*[dmg::RESOLUTION_X, dmg::RESOLUTION_Y]*/
                        &fb.buffer,
                    );
                    let texture_handle =
                        ui.load_texture("frame", image, egui::TextureOptions::default());
                    ui.image(&texture_handle);

                    ui.request_repaint();
                }
            });

            egui::Frame::new()
                .outer_margin(egui::Margin::same(5))
                .show(ui, |ui| {
                    let painter = ui.painter();
                    let rect = ui.available_rect_before_wrap();

                    painter.rect_stroke(
                        //rect.expand(4.0),
                        rect,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                    painter.rect_stroke(
                        rect.expand(4.0),
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin::same(MARGIN as i8))
                                .show(ui, |ui| {
                                    let rect = ui.available_rect_before_wrap();

                                    ui.set_min_size(egui::vec2(
                                        rect.width() - REG_PANEL_WIDTH,
                                        0.0,
                                    ));

                                    egui::Grid::new("grid_instructions").show(ui, |ui| {
                                        ui.label("(LD %a %b)");
                                        ui.end_row();

                                        ui.label("(LD %b %c)");
                                        ui.end_row();

                                        ui.label("(CP 144)");
                                        ui.end_row();
                                    });
                                });

                            egui::Frame::new()
                                .inner_margin(egui::Margin::same(MARGIN as i8))
                                .show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(REG_PANEL_WIDTH, rect.height()));

                                    let rect = ui.available_rect_before_wrap();

                                    // left border
                                    let painter = ui.painter();
                                    let left_border_rect = egui::Rect::from_min_max(
                                        egui::pos2(rect.min.x, rect.min.y - MARGIN),
                                        egui::pos2(rect.min.x + 4.0, rect.max.y - MARGIN),
                                    );

                                    painter.rect_filled(
                                        left_border_rect,
                                        0.0,
                                        egui::Color32::WHITE,
                                    );
                                    ui.add_space(9.0);
                                    egui::Grid::new("grid_registers").show(ui, |ui| {
                                        ui.label("ab 0x6654");
                                        ui.end_row();

                                        ui.label("cd 0x7654");
                                        ui.end_row();

                                        ui.label("ef 0x8975");
                                        ui.end_row();

                                        ui.label("");
                                        ui.end_row();

                                        ui.label("z=0");
                                        ui.end_row();

                                        ui.label("n=0");
                                        ui.end_row();

                                        ui.label("h=0");
                                        ui.end_row();

                                        ui.label("c=0");
                                        ui.end_row();
                                    });
                                });
                        });
                    });
                });
        });
    }
}
