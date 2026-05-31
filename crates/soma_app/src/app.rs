use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use egui::{Button, CentralPanel, FontDefinitions, Panel, ScrollArea};
use libsoma::{
    ROM,
    sm83::{self, Register},
};
use psy::arch::sm83::Sm83Instr;

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum StepControl {
    Break,
    BreakAt(u16),
    NextStep,
    Run,
}

/// Shared state between the emulator and the debugger
pub struct DebuggerSharedState {
    pub register: Register,
    pub tile_map_1: [u8; 32 * 32],
    pub step_control: StepControl,
}

impl DebuggerSharedState {
    pub fn new(step_control: StepControl) -> DebuggerSharedState {
        DebuggerSharedState {
            register: Register::zero(),
            tile_map_1: [0; 32 * 32],
            step_control,
        }
    }
}

#[derive(PartialEq)]
enum RegValueDisplay {
    Hex,
    Decimal,
    Binary,
}

/// Non shared state.
struct DebuggerState {
    pub disassemble_cache: HashMap<u16, &'static Sm83Instr>,
    pub reg_value_display: RegValueDisplay,
}

impl DebuggerState {
    pub fn new() -> DebuggerState {
        DebuggerState {
            disassemble_cache: HashMap::new(),
            reg_value_display: RegValueDisplay::Hex,
        }
    }
}

pub struct SomaApp<'a> {
    fb: Arc<RwLock<FrameBuffer>>,
    debug: Option<Debug<'a>>,
}

pub struct Debug<'a> {
    shared_state: Arc<RwLock<DebuggerSharedState>>,
    debugger_state: DebuggerState,
    rom: ROM<'a>,
}

impl<'a> Debug<'a> {
    pub fn new(shared_state: Arc<RwLock<DebuggerSharedState>>, rom: ROM<'a>) -> Debug<'a> {
        Debug {
            shared_state,
            debugger_state: DebuggerState::new(),
            rom,
        }
    }
}

impl<'a> SomaApp<'a> {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        fb: Arc<RwLock<FrameBuffer>>,
        debug: Option<Debug<'a>>,
    ) -> SomaApp<'a> {
        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        SomaApp { fb, debug }
    }

    /// Find n confirmed instructions from the disassemble execution cache.
    /// The Vec returned will always be of size n, places where no instructions
    /// could be found will be filled with None,
    fn n_instr(
        &self,
        start: u16,
        n: usize,
        forward: bool,
    ) -> Vec<(u16, Option<&'static Sm83Instr>)> {
        let dir = if forward { 1 } else { -1 };
        let mut result = Vec::with_capacity(n);

        let mut pc = start as i16 + dir;
        'instr: for _ in 0..n {
            for z in 0..3 {
                // a SM83 instruction can be max 3 bytes long
                let may_instr = self
                    .debug
                    .as_ref()
                    .expect("debug")
                    .debugger_state
                    .disassemble_cache
                    .get(&((pc + (z * dir)) as u16));
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

    fn instr_row_from_state(&self, ui: &mut egui::Ui, loc: u16, mark_halt: bool, rom: &ROM) {
        let may_instr = self
            .debug
            .as_ref()
            .unwrap()
            .debugger_state
            .disassemble_cache
            .get(&loc);
        if let Some(instr) = may_instr {
            instr_row(ui, loc, mark_halt, rom, Some(*instr));
        } else {
            instr_row(ui, loc, mark_halt, rom, None);
        };
    }

    fn val_u8(&self, v: u8) -> String {
        match self
            .debug
            .as_ref()
            .unwrap()
            .debugger_state
            .reg_value_display
        {
            RegValueDisplay::Binary => {
                format!("{:08b}", v)
            }
            RegValueDisplay::Hex => {
                format!("{:02X}", v)
            }
            RegValueDisplay::Decimal => {
                format!("{}", v)
            }
        }
    }

    fn val_u16(&self, v: u16) -> String {
        match self
            .debug
            .as_ref()
            .unwrap()
            .debugger_state
            .reg_value_display
        {
            RegValueDisplay::Binary => {
                format!("{:016b}", v)
            }
            RegValueDisplay::Hex => {
                format!("{:04X}", v)
            }
            RegValueDisplay::Decimal => {
                format!("{}", v)
            }
        }
    }
}

const REG_PANEL_WIDTH: f32 = 210.0;
const MARGIN: f32 = 5.0;

impl<'a> eframe::App for SomaApp<'a> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default().show_inside(ui, |ui| {
            // UI parts (screen + debug info)
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    let fb = self.fb.read().unwrap();

                    if fb.needs_update {
                        let image = egui::ColorImage::from_rgb(
                            [256, 256], /*[dmg::RESOLUTION_X, dmg::RESOLUTION_Y]*/
                            &fb.buffer,
                        );
                        let texture_handle =
                            ui.load_texture("frame", image, egui::TextureOptions::default());
                        ui.image(&texture_handle);

                        // TODO reset needs_update (rename to dirty) if the fb was converted and
                        // cache the image.

                        ui.request_repaint();
                    }

                    if self.debug.is_some() {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add(Button::selectable(true, "Tile Map"));
                            });
                            ScrollArea::vertical().show(ui, |ui| {
                                egui::Grid::new("grid_tilemap")
                                    .min_col_width(0.0)
                                    .min_row_height(0.0)
                                    .show(ui, |ui| {
                                        ui.style_mut().text_styles.insert(
                                            egui::TextStyle::Body,
                                            egui::FontId::new(8.0, egui::FontFamily::Proportional),
                                        );

                                        let shared_state = self
                                            .debug
                                            .as_ref()
                                            .unwrap()
                                            .shared_state
                                            .read()
                                            .unwrap();

                                        for x in 0..32 {
                                            for y in 0..32 {
                                                ui.label(format!(
                                                    "{:03}\u{2009}",
                                                    shared_state.tile_map_1[y * 32 + x]
                                                ));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        });
                    }
                });
            });

            if self.debug.is_some() {
                Panel::bottom("bottom").show_inside(ui, |ui| {
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
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        egui::Frame::new()
                                            .inner_margin(egui::Margin::same(MARGIN as i8))
                                            .show(ui, |ui| {
                                                let rect = ui.available_rect_before_wrap();

                                                ui.set_min_size(egui::vec2(
                                                    rect.width() - REG_PANEL_WIDTH,
                                                    0.0,
                                                ));

                                                {
                                                    let debug = self.debug.as_mut().unwrap();
                                                    let shared_state =
                                                        debug.shared_state.read().unwrap();
                                                    let pc = shared_state.register.pc;
                                                    let may_pc_instr = debug
                                                        .debugger_state
                                                        .disassemble_cache
                                                        .get(&pc);
                                                    if may_pc_instr.is_none() {
                                                        let instr = psy::arch::sm83::decode(
                                                            debug.rom.read_u8(pc as usize),
                                                        );
                                                        debug
                                                            .debugger_state
                                                            .disassemble_cache
                                                            .insert(pc, instr);
                                                    };
                                                }

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        if ui.button("Step").clicked() {
                                                            let mut shared_state = self
                                                                .debug
                                                                .as_ref()
                                                                .unwrap()
                                                                .shared_state
                                                                .write()
                                                                .unwrap();
                                                            shared_state.step_control =
                                                                StepControl::NextStep;
                                                        }
                                                    });

                                                    egui::Grid::new("grid_instructions")
                                                        .min_col_width(0.0)
                                                        .show(ui, |ui| {
                                                            let shared_state = self
                                                                .debug
                                                                .as_ref()
                                                                .unwrap()
                                                                .shared_state
                                                                .read()
                                                                .unwrap();
                                                            let pc = shared_state.register.pc;

                                                            let instr_before = self.n_instr(
                                                                pc, 5, false, /*backward*/
                                                            );
                                                            let instr_after = self.n_instr(
                                                                pc, 5, true, /*forward*/
                                                            );

                                                            for loc_instr in instr_before {
                                                                instr_row(
                                                                    ui,
                                                                    loc_instr.0,
                                                                    false,
                                                                    &self
                                                                        .debug
                                                                        .as_ref()
                                                                        .unwrap()
                                                                        .rom,
                                                                    loc_instr.1,
                                                                );
                                                            }

                                                            self.instr_row_from_state(
                                                                ui,
                                                                pc,
                                                                true,
                                                                &self.debug.as_ref().unwrap().rom,
                                                            );

                                                            for loc_instr in instr_after {
                                                                instr_row(
                                                                    ui,
                                                                    loc_instr.0,
                                                                    false,
                                                                    &self
                                                                        .debug
                                                                        .as_ref()
                                                                        .unwrap()
                                                                        .rom,
                                                                    loc_instr.1,
                                                                );
                                                            }
                                                        });
                                                });
                                            });

                                        egui::Frame::new()
                                            .inner_margin(egui::Margin::same(MARGIN as i8))
                                            .show(ui, |ui| {
                                                ui.set_min_size(egui::vec2(
                                                    REG_PANEL_WIDTH,
                                                    rect.height(),
                                                ));

                                                let rect = ui.available_rect_before_wrap();

                                                // left border
                                                let painter = ui.painter();
                                                let left_border_rect = egui::Rect::from_min_max(
                                                    egui::pos2(rect.min.x, rect.min.y - MARGIN),
                                                    egui::pos2(
                                                        rect.min.x + 4.0,
                                                        rect.max.y - MARGIN,
                                                    ),
                                                );

                                                painter.rect_filled(
                                                    left_border_rect,
                                                    0.0,
                                                    egui::Color32::WHITE,
                                                );
                                                ui.add_space(9.0);

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        if ui
                                                            .add(Button::selectable(
                                                                self.debug
                                                                    .as_ref()
                                                                    .unwrap()
                                                                    .debugger_state
                                                                    .reg_value_display
                                                                    == RegValueDisplay::Hex,
                                                                "Hex",
                                                            ))
                                                            .clicked()
                                                        {
                                                            self.debug
                                                                .as_mut()
                                                                .unwrap()
                                                                .debugger_state
                                                                .reg_value_display =
                                                                RegValueDisplay::Hex
                                                        };
                                                        if ui
                                                            .add(Button::selectable(
                                                                self.debug
                                                                    .as_ref()
                                                                    .unwrap()
                                                                    .debugger_state
                                                                    .reg_value_display
                                                                    == RegValueDisplay::Binary,
                                                                "Bin",
                                                            ))
                                                            .clicked()
                                                        {
                                                            self.debug
                                                                .as_mut()
                                                                .unwrap()
                                                                .debugger_state
                                                                .reg_value_display =
                                                                RegValueDisplay::Binary
                                                        };
                                                        if ui
                                                            .add(Button::selectable(
                                                                self.debug
                                                                    .as_ref()
                                                                    .unwrap()
                                                                    .debugger_state
                                                                    .reg_value_display
                                                                    == RegValueDisplay::Decimal,
                                                                "Dec",
                                                            ))
                                                            .clicked()
                                                        {
                                                            self.debug
                                                                .as_mut()
                                                                .unwrap()
                                                                .debugger_state
                                                                .reg_value_display =
                                                                RegValueDisplay::Decimal
                                                        };
                                                    });

                                                    let shared_state = self
                                                        .debug
                                                        .as_ref()
                                                        .unwrap()
                                                        .shared_state
                                                        .read()
                                                        .unwrap();
                                                    egui::Grid::new("grid_registers").show(
                                                        ui,
                                                        |ui| {
                                                            ui.label("a");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.a,
                                                                ),
                                                            );
                                                            ui.end_row();

                                                            ui.label("b");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.b,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("c");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.c,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("bc");
                                                            ui.label(self.val_u16(
                                                                shared_state.register.bc(),
                                                            ));
                                                            ui.end_row();

                                                            ui.label("d");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.d,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("e");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.e,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("de");
                                                            ui.label(self.val_u16(
                                                                shared_state.register.de(),
                                                            ));
                                                            ui.end_row();

                                                            ui.label("h");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.h,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("l");
                                                            ui.label(
                                                                self.val_u8(
                                                                    shared_state.register.l,
                                                                ),
                                                            );
                                                            ui.end_row();
                                                            ui.label("hl");
                                                            ui.label(self.val_u16(
                                                                shared_state.register.hl(),
                                                            ));
                                                            ui.end_row();

                                                            ui.label("sp");
                                                            ui.label(
                                                                self.val_u16(
                                                                    shared_state.register.sp,
                                                                ),
                                                            );
                                                            ui.end_row();

                                                            ui.label("pc");
                                                            ui.label(
                                                                self.val_u16(
                                                                    shared_state.register.pc,
                                                                ),
                                                            );
                                                            ui.end_row();

                                                            ui.label("z");
                                                            ui.label(flag(
                                                                shared_state.register.f,
                                                                sm83::Z,
                                                            ));
                                                            ui.end_row();

                                                            ui.label("n");
                                                            ui.label(flag(
                                                                shared_state.register.f,
                                                                sm83::N,
                                                            ));
                                                            ui.end_row();

                                                            ui.label("h");
                                                            ui.label(flag(
                                                                shared_state.register.f,
                                                                sm83::H,
                                                            ));
                                                            ui.end_row();

                                                            ui.label("c");
                                                            ui.label(flag(
                                                                shared_state.register.f,
                                                                sm83::C,
                                                            ));
                                                            ui.end_row();
                                                        },
                                                    );
                                                });
                                            });
                                    },
                                );
                            });
                        });
                });
            }
        });
    }
}

fn flag(v: u8, m: u8) -> &'static str {
    if v & m == 0 { "0" } else { "1" }
}

fn instr_row(
    ui: &mut egui::Ui,
    loc: u16,
    mark_halt: bool,
    rom: &ROM,
    may_instr: Option<&Sm83Instr>,
) {
    if mark_halt {
        ui.label(egui_phosphor::regular::PLAY);
    } else {
        ui.label("");
    }

    ui.label(format!("0x{:X}", loc));

    let instr_text = if let Some(instr) = may_instr {
        let loc_u = loc as usize;
        let text = instr.text(Some(&rom.data[loc_u..(loc_u + 3)]));
        if instr.op_code == psy::arch::sm83::INSTR_INVALID.op_code {
            format!("{} op_code=0x{:x}", text, rom.data[loc_u])
        } else {
            text
        }
    } else {
        "???".to_string()
    };

    ui.label(instr_text);
    ui.end_row();
}
