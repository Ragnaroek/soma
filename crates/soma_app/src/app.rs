use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use egui::{CentralPanel, FontDefinitions};
use libsoma::{ROM, sm83::Register};
use psy::arch::sm83::{self, Sm83Instr};

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum StepControl {
    Break,
    NextStep,
    Run,
}

pub struct DebuggerState {
    pub register: Register,
    pub step_control: StepControl,
    pub disassemble: HashMap<u16, &'static Sm83Instr>,
}

impl DebuggerState {
    pub fn new() -> DebuggerState {
        DebuggerState {
            register: Register::zero(),
            step_control: StepControl::Break,
            disassemble: HashMap::new(),
        }
    }
}

pub struct SomaApp<'a> {
    fb: Arc<RwLock<FrameBuffer>>,
    debugger_state: Arc<RwLock<DebuggerState>>,
    rom: ROM<'a>,
}

impl<'a> SomaApp<'a> {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        fb: Arc<RwLock<FrameBuffer>>,
        debugger_state: Arc<RwLock<DebuggerState>>,
        rom: ROM<'a>,
    ) -> SomaApp<'a> {
        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        SomaApp {
            fb,
            debugger_state,
            rom,
        }
    }
}

const REG_PANEL_WIDTH: f32 = 120.0;
const MARGIN: f32 = 5.0;

impl<'a> eframe::App for SomaApp<'a> {
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

                                    {
                                        let mut debug_state = self.debugger_state.write().unwrap();
                                        let pc = debug_state.register.pc;
                                        let may_pc_instr = debug_state.disassemble.get(&pc);
                                        if may_pc_instr.is_none() {
                                            let instr = sm83::decode(self.rom.read_u8(pc as usize));
                                            debug_state.disassemble.insert(pc, instr);
                                        };
                                    }

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button("Step").clicked() {
                                                let mut debug_state =
                                                    self.debugger_state.write().unwrap();
                                                debug_state.step_control = StepControl::NextStep;
                                            }
                                        });

                                        egui::Grid::new("grid_instructions")
                                            .min_col_width(0.0)
                                            .show(ui, |ui| {
                                                let debug_state =
                                                    self.debugger_state.read().unwrap();
                                                let pc = debug_state.register.pc;

                                                let instr_before = n_instr(
                                                    pc,
                                                    5,
                                                    false, /*backward*/
                                                    &debug_state,
                                                );
                                                let instr_after = n_instr(
                                                    pc,
                                                    5,
                                                    true, /*forward*/
                                                    &debug_state,
                                                );

                                                for loc_instr in instr_before {
                                                    instr_row(ui, loc_instr.0, false, loc_instr.1);
                                                }

                                                instr_row_from_state(ui, pc, true, &debug_state);

                                                for loc_instr in instr_after {
                                                    instr_row(ui, loc_instr.0, false, loc_instr.1);
                                                }
                                            });
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

fn instr_row_from_state(ui: &mut egui::Ui, loc: u16, mark_halt: bool, debug_state: &DebuggerState) {
    let may_instr = debug_state.disassemble.get(&loc);
    if let Some(instr) = may_instr {
        instr_row(ui, loc, mark_halt, Some(*instr));
    } else {
        instr_row(ui, loc, mark_halt, None);
    };
}

fn instr_row(ui: &mut egui::Ui, loc: u16, mark_halt: bool, may_instr: Option<&Sm83Instr>) {
    if mark_halt {
        ui.label(egui_phosphor::regular::PLAY);
    } else {
        ui.label("");
    }

    ui.label(format!("0x{:X}", loc));

    let instr_text = if let Some(instr) = may_instr {
        &instr.text(None)
    } else {
        "???"
    };

    ui.label(instr_text);
    ui.end_row();
}

/// Find n confirmed instructions from the disassemble execution cache.
/// The Vec returned will always be of size n, places where no instructions
/// could be found will be filled with None,
fn n_instr(
    start: u16,
    n: usize,
    forward: bool,
    debug_state: &DebuggerState,
) -> Vec<(u16, Option<&'static Sm83Instr>)> {
    let dir = if forward { 1 } else { -1 };
    let mut result = Vec::with_capacity(n);

    let mut pc = start as i16 + dir;
    'instr: for _ in 0..n {
        for z in 0..3 {
            // a SM83 instruction can be max 3 bytes long
            let may_instr = debug_state.disassemble.get(&((pc + (z * dir)) as u16));
            if let Some(instr) = may_instr {
                result.push((pc as u16, Some(*instr)));
                pc += (z + 1) * dir;
                continue 'instr;
            }
        }
        // no instruction found
        result.push((pc as u16, None));
        pc += dir;
    }

    if !forward {
        result.reverse();
    }
    result
}
