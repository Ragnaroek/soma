use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashMap, sync::Arc};

use egui::{Button, FontDefinitions, Frame, Pos2, Rect, ScrollArea};
use psy::arch::sm83::Sm83Instr;

use libsoma::dmg::DMG;
use libsoma::rom::ROM;
use libsoma::sm83;
use std::time::Instant;

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

pub struct Emulation {
    dmg: RwLock<DMG<Instant>>,
    step_control: RwLock<StepControl>,
}

impl Emulation {
    pub fn new(dmg: DMG<Instant>, init_step: StepControl) -> Emulation {
        Emulation {
            dmg: RwLock::new(dmg),
            step_control: RwLock::new(init_step),
        }
    }

    pub fn dmg_write_lock<'a>(&'a self) -> RwLockWriteGuard<'a, DMG<Instant>> {
        self.dmg.write().expect("dmg write lock")
    }

    pub fn dmg_read_lock<'a>(&'a self) -> RwLockReadGuard<'a, DMG<Instant>> {
        self.dmg.read().expect("dmg read lock")
    }

    pub fn step_control(&self) -> StepControl {
        *self.step_control.read().expect("step_control lock")
    }

    pub fn set_step_control(&self, step: StepControl) {
        *self.step_control.write().expect("step_control lock") = step;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum StepControl {
    Break,
    BreakAt(u16),
    NextStep,
    Run,
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

pub struct SomaApp {
    fb: Arc<RwLock<FrameBuffer>>,
    debugger: Option<Debugger>,
}

pub struct Debugger {
    emulator: Arc<Emulation>, // TOOD rename field to emu
    state: DebuggerState,
}

impl Debugger {
    pub fn new(emulator: Arc<Emulation>) -> Debugger {
        Debugger {
            emulator,
            state: DebuggerState::new(),
        }
    }
}

impl SomaApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        fb: Arc<RwLock<FrameBuffer>>,
        debugger: Option<Debugger>,
    ) -> SomaApp {
        let mut fonts = FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        SomaApp { fb, debugger }
    }

    fn render_memory_view(&self, ui: &mut egui::Ui, width: f32, height: f32) {
        if let Some(debugger) = self.debugger.as_ref() {
            Frame::new().show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(height);

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

                                let dmg = debugger.emulator.dmg_read_lock();
                                for x in 0..32 {
                                    for y in 0..32 {
                                        ui.label(format!(
                                            "{:03}\u{2009}",
                                            dmg.mc.read(0x9800 + y * 32 + x)
                                        ));
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                });
            });
        }
    }

    fn render_asm_view(&mut self, ui: &mut egui::Ui, available_rect: Rect) {
        if self.debugger.is_some() {
            let painter = ui.painter();
            painter.rect_stroke(
                available_rect,
                0.0,
                egui::Stroke::new(2.0f32, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                available_rect.shrink(4.0),
                0.0,
                egui::Stroke::new(1.0f32, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );

            let asm_margin = 2.0;
            let content_area = available_rect.shrink(8.0); //4.0 + 4.0 additional margin for the content
            let (asm_area, reg_area) = content_area
                .split_left_right_at_x(content_area.min.x + content_area.width() - REG_PANEL_WIDTH);
            let (button_area, asm_code_area) =
                asm_area.split_top_bottom_at_y(asm_area.min.y + 30.0);

            // button bar
            egui::Area::new(egui::Id::new("button_bar"))
                .fixed_pos(button_area.min)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Step").clicked() {
                            self.debugger
                                .as_ref()
                                .unwrap()
                                .emulator
                                .set_step_control(StepControl::NextStep);
                        }
                        if ui.button("Run").clicked() {
                            self.debugger
                                .as_ref()
                                .unwrap()
                                .emulator
                                .set_step_control(StepControl::Run);
                        }
                    });
                });

            egui::Area::new(egui::Id::new("asm_code"))
                .fixed_pos(asm_code_area.min)
                .show(ui, |ui| {
                    {
                        let debug = self.debugger.as_mut().unwrap();
                        let dmg = debug.emulator.dmg_read_lock();
                        let pc = dmg.sm83.pc();
                        let may_pc_instr = debug.state.disassemble_cache.get(&pc);
                        if may_pc_instr.is_none() {
                            let instr = psy::arch::sm83::decode(dmg.mc.read(pc));
                            debug.state.disassemble_cache.insert(pc, instr);
                        };
                    }

                    ScrollArea::vertical()
                        .max_height(asm_code_area.height())
                        .max_width(asm_code_area.width())
                        .show(ui, |ui| {
                            egui::Grid::new("grid_instructions")
                                .min_col_width(0.0)
                                .show(ui, |ui| {
                                    let dmg =
                                        self.debugger.as_ref().unwrap().emulator.dmg_read_lock();
                                    let pc = dmg.sm83.pc();
                                    let rom = dmg.mc.rom.as_ref().expect("ROM");

                                    let instr_before = self.n_instr(pc, 5, false /*backward*/);
                                    let instr_after = self.n_instr(pc, 5, true /*forward*/);

                                    for loc_instr in instr_before {
                                        instr_row(ui, loc_instr.0, false, rom, loc_instr.1);
                                    }

                                    self.instr_row_from_state(ui, pc, true, rom);

                                    for loc_instr in instr_after {
                                        instr_row(ui, loc_instr.0, false, rom, loc_instr.1);
                                    }
                                });
                        });
                });

            let (_, reg_area_inner) = reg_area.split_left_right_at_x(reg_area.min.x + 8.0);
            egui::Area::new(egui::Id::new("reg"))
                .fixed_pos(reg_area.min)
                .show(ui, |ui| {
                    // left border
                    let painter = ui.painter();
                    let left_border_rect = egui::Rect::from_min_max(
                        egui::pos2(reg_area.min.x, reg_area.min.y + 2.0 - asm_margin),
                        egui::pos2(reg_area.min.x + 4.0, reg_area.max.y - asm_margin),
                    );

                    painter.rect_filled(left_border_rect, 0.0, egui::Color32::WHITE);

                    egui::Area::new(egui::Id::new("reg_inner"))
                        .fixed_pos(reg_area_inner.min)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(Button::selectable(
                                            self.debugger.as_ref().unwrap().state.reg_value_display
                                                == RegValueDisplay::Hex,
                                            "Hex",
                                        ))
                                        .clicked()
                                    {
                                        self.debugger.as_mut().unwrap().state.reg_value_display =
                                            RegValueDisplay::Hex
                                    };
                                    if ui
                                        .add(Button::selectable(
                                            self.debugger.as_ref().unwrap().state.reg_value_display
                                                == RegValueDisplay::Binary,
                                            "Bin",
                                        ))
                                        .clicked()
                                    {
                                        self.debugger.as_mut().unwrap().state.reg_value_display =
                                            RegValueDisplay::Binary
                                    };
                                    if ui
                                        .add(Button::selectable(
                                            self.debugger.as_ref().unwrap().state.reg_value_display
                                                == RegValueDisplay::Decimal,
                                            "Dec",
                                        ))
                                        .clicked()
                                    {
                                        self.debugger.as_mut().unwrap().state.reg_value_display =
                                            RegValueDisplay::Decimal
                                    };
                                });

                                let dmg = self.debugger.as_ref().unwrap().emulator.dmg_read_lock();
                                egui::Grid::new("grid_registers").show(ui, |ui| {
                                    ui.label("a");
                                    ui.label(self.val_u8(dmg.sm83.reg.a));
                                    ui.end_row();

                                    ui.label("b");
                                    ui.label(self.val_u8(dmg.sm83.reg.b));
                                    ui.end_row();
                                    ui.label("c");
                                    ui.label(self.val_u8(dmg.sm83.reg.c));
                                    ui.end_row();
                                    ui.label("bc");
                                    ui.label(self.val_u16(dmg.sm83.reg.bc()));
                                    ui.end_row();

                                    ui.label("d");
                                    ui.label(self.val_u8(dmg.sm83.reg.d));
                                    ui.end_row();
                                    ui.label("e");
                                    ui.label(self.val_u8(dmg.sm83.reg.e));
                                    ui.end_row();
                                    ui.label("de");
                                    ui.label(self.val_u16(dmg.sm83.reg.de()));
                                    ui.end_row();

                                    ui.label("h");
                                    ui.label(self.val_u8(dmg.sm83.reg.h));
                                    ui.end_row();
                                    ui.label("l");
                                    ui.label(self.val_u8(dmg.sm83.reg.l));
                                    ui.end_row();
                                    ui.label("hl");
                                    ui.label(self.val_u16(dmg.sm83.reg.hl()));
                                    ui.end_row();

                                    ui.label("sp");
                                    ui.label(self.val_u16(dmg.sm83.reg.sp));
                                    ui.end_row();

                                    ui.label("pc");
                                    ui.label(self.val_u16(dmg.sm83.reg.pc));
                                    ui.end_row();

                                    ui.label("z");
                                    ui.label(flag(dmg.sm83.reg.f, sm83::Z));
                                    ui.end_row();

                                    ui.label("n");
                                    ui.label(flag(dmg.sm83.reg.f, sm83::N));
                                    ui.end_row();

                                    ui.label("h");
                                    ui.label(flag(dmg.sm83.reg.f, sm83::H));
                                    ui.end_row();

                                    ui.label("c");
                                    ui.label(flag(dmg.sm83.reg.f, sm83::C));
                                    ui.end_row();
                                });
                            });
                        });
                });
        }
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
                    .debugger
                    .as_ref()
                    .expect("debug")
                    .state
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
            .debugger
            .as_ref()
            .expect("debugger")
            .state
            .disassemble_cache
            .get(&loc);
        if let Some(instr) = may_instr {
            instr_row(ui, loc, mark_halt, rom, Some(*instr));
        } else {
            instr_row(ui, loc, mark_halt, rom, None);
        };
    }

    fn val_u8(&self, v: u8) -> String {
        match self.debugger.as_ref().unwrap().state.reg_value_display {
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
        match self.debugger.as_ref().unwrap().state.reg_value_display {
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
const DISPLAY_HEIGHT: f32 = 256.0;
const DISPLAY_WIDTH: f32 = 256.0;
const MARGIN: f32 = 5.0;

impl eframe::App for SomaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let screen_size = ui.ctx().content_rect();

        //let painter = ui.painter();

        egui::Area::new(egui::Id::new("display"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ui, |ui| {
                let fb = self.fb.read().unwrap();

                if fb.needs_update {
                    let image = egui::ColorImage::from_rgb(
                        [DISPLAY_WIDTH as usize, DISPLAY_HEIGHT as usize],
                        &fb.buffer,
                    );
                    let texture_handle =
                        ui.load_texture("frame", image, egui::TextureOptions::default());
                    ui.image(&texture_handle);

                    // TODO reset needs_update (rename to dirty) if the fb was converted and
                    // cache the image.

                    ui.request_repaint();
                }
            });

        let tile_height = DISPLAY_HEIGHT;
        let tile_width = screen_size.width() - (DISPLAY_WIDTH + MARGIN);
        egui::Area::new(egui::Id::new("tile_view"))
            .fixed_pos(egui::pos2(DISPLAY_WIDTH + MARGIN, 0.0))
            .show(ui, |ui| {
                self.render_memory_view(ui, tile_width, tile_height);
            });
        /*
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(0.0, DISPLAY_HEIGHT + MARGIN),
                Pos2::new(screen_size.width(), screen_size.height()),
            ),
            0.0,
            egui::Color32::RED,
        );*/

        let asm_rect = Rect::from_min_max(
            Pos2::new(0.0, DISPLAY_HEIGHT + MARGIN),
            Pos2::new(screen_size.width(), screen_size.height()),
        );
        egui::Area::new(egui::Id::new("asm_view"))
            .fixed_pos(egui::pos2(0.0, DISPLAY_HEIGHT + MARGIN))
            .show(ui, |ui| {
                //let painter = ui.painter();
                //painter.rect_filled(asm_rect, 0.0, egui::Color32::GREEN);
                self.render_asm_view(ui, asm_rect);
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
