use std::collections::HashSet;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashMap, sync::Arc};

use egui::{Button, Color32, FontDefinitions, Frame, Grid, Pos2, Rect, ScrollArea, Stroke};
use psy::arch::sm83::{MAX_INSTRUCTION_BYTE_LENGTH, Sm83Instr};

use libsoma::dmg::{self, DMG};
use libsoma::rom::ROM;
use libsoma::sm83;
use std::time::Instant;

const REG_PANEL_WIDTH: f32 = 210.0;
const STACK_PANEL_WIDTH: f32 = 210.0;

const MARGIN: f32 = 5.0;
const MARGIN_ASM: f32 = 4.0;

const ASM_CODE_ROW_HEIGHT: f32 = 18.0;
const ASM_OVERVIEW_WIDTH: f32 = 20.0;

const COLOUR_UNCONFIRMED_INSTR: Color32 = Color32::from_rgb(0xFF, 0x98, 0x00);
const COLOUR_VIEWPORT_OVERVIEW: Color32 = Color32::from_rgba_premultiplied(0x28, 0x28, 0x28, 0x80);

pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub needs_update: bool,
}

pub struct Emulation {
    dmg: RwLock<DMG<Instant>>,
    step_control: RwLock<StepControl>,
    breakpoints: RwLock<HashSet<u16>>,
    disassemble_cache: RwLock<HashMap<u16, DisassembleInstr>>,
}

impl Emulation {
    pub fn new(dmg: DMG<Instant>, init_step: StepControl) -> Emulation {
        Emulation {
            dmg: RwLock::new(dmg),
            step_control: RwLock::new(init_step),
            breakpoints: RwLock::new(HashSet::new()),
            disassemble_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn dmg_write_lock<'a>(&'a self) -> RwLockWriteGuard<'a, DMG<Instant>> {
        self.dmg.write().expect("dmg write lock")
    }

    pub fn dmg_read_lock<'a>(&'a self) -> RwLockReadGuard<'a, DMG<Instant>> {
        self.dmg.read().expect("dmg read lock")
    }

    pub fn disassemble_cache_write_lock<'a>(
        &'a self,
    ) -> RwLockWriteGuard<'a, HashMap<u16, DisassembleInstr>> {
        self.disassemble_cache
            .write()
            .expect("disassemble cache write lock")
    }

    pub fn step_control(&self) -> StepControl {
        *self.step_control.read().expect("step_control lock")
    }

    pub fn set_step_control(&self, step: StepControl) {
        *self.step_control.write().expect("step_control lock") = step;
    }

    pub fn toggle_breakpoint(&self, loc: u16) {
        let mut breakpoints = self.breakpoints.write().expect("breakpoint lock");
        if breakpoints.contains(&loc) {
            breakpoints.remove(&loc);
        } else {
            breakpoints.insert(loc);
        }
    }

    pub fn has_breakpoint_at(&self, loc: u16) -> bool {
        self.breakpoints
            .read()
            .expect("breakpoint lock")
            .contains(&loc)
    }
}

// Protocol between the emulation loop and a debugger.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StepControl {
    // Ignore all breakpoint and do a normal run of the emulation
    Run,
    // Halt the emulation
    Halt,
    // If in Halt continue Run.
    Resume,
    // Do one step and set to to 'Halt' after this step.
    NextStep,
}

#[derive(PartialEq)]
enum RegValueDisplay {
    Hex,
    Decimal,
    Binary,
}

#[derive(Copy, Clone)]
pub struct DisassembleInstr {
    pub confirmed: bool,
    pub instr: &'static Sm83Instr,
}

struct JumpToDialogState {
    show: bool,
    input_text: String,
}

struct DebuggerState {
    pub reg_value_display: RegValueDisplay,
    pub asm_view_at: AsmViewAt,
    pub jump_to_dialog_state: JumpToDialogState,
}

enum AsmViewAt {
    // follow the program counter of the emulator
    PC,
    // show a fixed address and ignore the program counter
    FixedPos(usize),
}

impl DebuggerState {
    pub fn new() -> DebuggerState {
        DebuggerState {
            reg_value_display: RegValueDisplay::Hex,
            asm_view_at: AsmViewAt::PC,
            jump_to_dialog_state: JumpToDialogState {
                show: false,
                input_text: "".to_string(),
            },
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
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
        cc.egui_ctx.set_fonts(fonts);

        SomaApp { fb, debugger }
    }

    fn handle_keys(&mut self, ui: &mut egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::J) && i.modifiers.ctrl) {
            if let Some(debugger) = &mut self.debugger {
                debugger.state.jump_to_dialog_state.show = true;
            }
        }
    }

    fn handle_dialogs(&mut self, ui: &mut egui::Ui) {
        if let Some(debugger) = &mut self.debugger {
            if debugger.state.jump_to_dialog_state.show {
                egui::Modal::new(egui::Id::new("my_modal")).show(ui, |ui| {
                    ui.heading("Enter address to jump to (hex value)");

                    let response = ui
                        .text_edit_singleline(&mut debugger.state.jump_to_dialog_state.input_text);
                    response.request_focus();

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Jump").clicked() {
                            let may_addr = usize::from_str_radix(
                                &debugger.state.jump_to_dialog_state.input_text,
                                16,
                            );
                            if let Ok(addr) = may_addr {
                                debugger.state.asm_view_at = AsmViewAt::FixedPos(addr);
                            }
                            // TODO show an error somewhere if address is invalid
                            debugger.state.jump_to_dialog_state.input_text.clear();
                            debugger.state.jump_to_dialog_state.show = false;
                        }

                        if ui.button("Cancel").clicked() {
                            debugger.state.jump_to_dialog_state.input_text.clear();
                            debugger.state.jump_to_dialog_state.show = false;
                        }
                    });
                });
            }
        }
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
                                            dmg.mc
                                                .read(0x9800 + y * 32 + x)
                                                .expect("tilemap value")
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
                available_rect.shrink(MARGIN_ASM),
                0.0,
                egui::Stroke::new(1.0f32, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );

            let asm_margin = 2.0;
            let content_area = available_rect.shrink(8.0); //4.0 + 4.0 additional margin for the content
            let (asm_full_area, reg_and_stack_area) = content_area.split_left_right_at_x(
                content_area.min.x + content_area.width() - REG_PANEL_WIDTH - STACK_PANEL_WIDTH,
            );
            let (reg_area, stack_area) = reg_and_stack_area
                .split_left_right_at_x(reg_and_stack_area.min.x + REG_PANEL_WIDTH);
            let (asm_area, asm_overview_area) =
                asm_full_area.split_left_right_at_x(asm_full_area.max.x - ASM_OVERVIEW_WIDTH);
            let (asm_overview_area, _) =
                asm_overview_area.split_left_right_at_x(asm_overview_area.max.x - 3.0);
            let (button_area, asm_code_area) =
                asm_area.split_top_bottom_at_y(asm_area.min.y + 30.0);

            // button bar
            egui::Area::new(egui::Id::new("button_bar"))
                .fixed_pos(button_area.min)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let current_control =
                            self.debugger.as_ref().unwrap().emulator.step_control();

                        let (step_enabled, resume_enabled) = match current_control {
                            StepControl::Run => (false, false),
                            StepControl::Halt => (true, true),
                            StepControl::NextStep => (false, false),
                            StepControl::Resume => (false, false),
                        };

                        if ui.add_enabled(step_enabled, Button::new("Step")).clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::S))
                        {
                            self.debugger
                                .as_ref()
                                .unwrap()
                                .emulator
                                .set_step_control(StepControl::NextStep);
                            // re-sync with the PC on next step
                            self.debugger.as_mut().unwrap().state.asm_view_at = AsmViewAt::PC;
                        }

                        if ui
                            .add_enabled(resume_enabled, Button::new("Resume"))
                            .clicked()
                        {
                            let emu = &self.debugger.as_ref().unwrap().emulator;
                            match current_control {
                                StepControl::Halt => {
                                    emu.set_step_control(StepControl::Resume);
                                }
                                _ => {
                                    unreachable!("only enabled for Breakpoint")
                                }
                            }
                        }
                    });
                });

            let num_asm_rows = ((asm_code_area.height() - 50.0) / ASM_CODE_ROW_HEIGHT) as usize;
            egui::Area::new(egui::Id::new("asm_code"))
                .fixed_pos(asm_code_area.min)
                .show(ui, |ui| {
                    egui::Grid::new("grid_instructions")
                        .min_col_width(0.0)
                        .min_row_height(ASM_CODE_ROW_HEIGHT)
                        .show(ui, |ui| {
                            let debugger = self.debugger.as_mut().expect("debugger");
                            let dmg = debugger.emulator.dmg_read_lock();
                            let mut dis_cache = debugger.emulator.disassemble_cache_write_lock();
                            let rom = dmg.mc.rom.as_ref().expect("ROM");

                            let (viewport_pos, confirmed) = match debugger.state.asm_view_at {
                                AsmViewAt::PC => (dmg.sm83.pc() as usize, true),
                                AsmViewAt::FixedPos(pos) => (pos, false),
                            };

                            let viewport_min = viewport_pos
                                .saturating_sub(num_asm_rows * MAX_INSTRUCTION_BYTE_LENGTH);

                            let viewport_max = (viewport_pos
                                + (num_asm_rows * MAX_INSTRUCTION_BYTE_LENGTH))
                                .min(rom.size());

                            // disassemble the instruction the view revolves around into the cache
                            disassemble_pc(viewport_pos as u16, &dmg, &mut dis_cache, confirmed);
                            predict_disassemble_around_pc(
                                viewport_min as u16,
                                viewport_max as u16,
                                &dmg,
                                &mut dis_cache,
                            );

                            // extract enough instructions from the cache
                            let instrs = instr_in_range(
                                &dis_cache,
                                viewport_min as u16,
                                viewport_max as u16,
                            );

                            let mut viewport_pos_ix = 0;
                            // find the ix of the centred pos
                            for i in 0..instrs.len() {
                                if instrs[i].0 == viewport_pos as u16 {
                                    viewport_pos_ix = i;
                                    break;
                                }
                            }

                            let max_rows_half = num_asm_rows / 2;

                            let mut start = viewport_pos_ix.saturating_sub(max_rows_half);
                            let mut end = (viewport_pos_ix + (num_asm_rows / 2)).min(instrs.len());

                            let dist_start = viewport_pos_ix - start;
                            let dist_end = end - viewport_pos_ix;

                            if dist_start < max_rows_half {
                                end = rom.size().min(end + (max_rows_half - dist_start));
                            }
                            if dist_end < max_rows_half {
                                start = start.saturating_sub(max_rows_half - dist_end)
                            }

                            for i in start..end {
                                let instr = &instrs[i];
                                let mark_halt = instr.0 == dmg.sm83.pc();
                                render_instr(
                                    ui,
                                    instr.1,
                                    instr.0,
                                    mark_halt,
                                    rom,
                                    &debugger.emulator,
                                );
                            }
                        });
                });

            let viewport_center = {
                let debugger = self.debugger.as_mut().expect("debugger");
                let dmg = debugger.emulator.dmg_read_lock();
                let rom_size = dmg.mc.rom.as_ref().expect("rom").size();

                let viewport_pos_pc = match debugger.state.asm_view_at {
                    AsmViewAt::PC => dmg.sm83.pc() as usize,
                    AsmViewAt::FixedPos(pos) => pos,
                };

                let byte_height = asm_overview_area.height() / rom_size as f32;
                let viewport_center = byte_height * viewport_pos_pc as f32;

                viewport_center + available_rect.min.y + MARGIN_ASM
            };

            egui::Area::new(egui::Id::new("asm_overview_bar"))
                .fixed_pos(asm_overview_area.min)
                .show(ui, |ui| {
                    ui.painter()
                        .rect_filled(asm_overview_area, 0.0, Color32::LIGHT_GRAY);
                    let rect = Rect::from_min_max(
                        Pos2::new(
                            asm_overview_area.min.x,
                            (viewport_center - 5.0).max(asm_overview_area.min.y),
                        ),
                        Pos2::new(
                            asm_overview_area.max.x,
                            (viewport_center + 6.0).min(asm_overview_area.max.y),
                        ),
                    );
                    ui.painter()
                        .rect_filled(rect, 0.0, COLOUR_VIEWPORT_OVERVIEW);
                    ui.painter().line(
                        vec![
                            Pos2::new(asm_overview_area.min.x, viewport_center),
                            Pos2::new(asm_overview_area.max.x, viewport_center),
                        ],
                        Stroke::new(1.0, Color32::BLACK),
                    );
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

                                    ui.label("ie");
                                    ui.label(format!("{}", dmg.sm83.reg.ie));
                                    ui.end_row();

                                    ui.label("ime");
                                    ui.label(format!("{}", dmg.sm83.reg.ime));
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

            let (_, stack_area_inner) = stack_area.split_left_right_at_x(stack_area.min.x + 8.0);
            egui::Area::new(egui::Id::new("stack"))
                .fixed_pos(stack_area.min)
                .show(ui, |ui| {
                    // left border
                    let painter = ui.painter();
                    let left_border_rect = egui::Rect::from_min_max(
                        egui::pos2(stack_area.min.x, stack_area.min.y + 2.0 - asm_margin),
                        egui::pos2(stack_area.min.x + 4.0, stack_area.max.y - asm_margin),
                    );

                    painter.rect_filled(left_border_rect, 0.0, egui::Color32::WHITE);

                    egui::Area::new(egui::Id::new("stack_inner"))
                        .fixed_pos(stack_area_inner.min)
                        .show(ui, |ui| {
                            ScrollArea::vertical().show(ui, |ui| {
                                Grid::new("stack_grid").show(ui, |ui| {
                                    let debugger = self.debugger.as_ref().unwrap();
                                    let dmg = debugger.emulator.dmg_read_lock();
                                    let sp = dmg.sm83.reg.sp;

                                    for addr in sp..(sp.saturating_add(100)) {
                                        ui.label(format!("0x{:X}", addr));
                                        ui.label(format!(
                                            "{:X}",
                                            dmg.mc.read(addr).expect("mem read")
                                        ));
                                        ui.end_row();
                                    }
                                });
                            });
                        });
                });
        }
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

impl eframe::App for SomaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_keys(ui);
        self.handle_dialogs(ui);

        let screen_size = ui.ctx().content_rect();

        egui::Area::new(egui::Id::new("display"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ui, |ui| {
                let fb = self.fb.read().unwrap();

                if fb.needs_update {
                    let image = egui::ColorImage::from_rgb(
                        [dmg::RESOLUTION_X, dmg::RESOLUTION_Y],
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

        let tile_height = dmg::RESOLUTION_Y as f32;
        let tile_width = screen_size.width() - (dmg::RESOLUTION_X as f32 + MARGIN);
        egui::Area::new(egui::Id::new("tile_view"))
            .fixed_pos(egui::pos2(dmg::RESOLUTION_X as f32 + MARGIN, 0.0))
            .show(ui, |ui| {
                self.render_memory_view(ui, tile_width, tile_height);
            });

        let asm_rect = Rect::from_min_max(
            Pos2::new(0.0, dmg::RESOLUTION_Y as f32 + MARGIN),
            Pos2::new(screen_size.width(), screen_size.height()),
        );
        egui::Area::new(egui::Id::new("asm_view"))
            .fixed_pos(egui::pos2(0.0, dmg::RESOLUTION_Y as f32 + MARGIN))
            .show(ui, |ui| {
                self.render_asm_view(ui, asm_rect);
            });
    }
}

fn flag(v: u8, m: u8) -> &'static str {
    if v & m == 0 { "0" } else { "1" }
}

fn render_instr(
    ui: &mut egui::Ui,
    may_instr: Option<&DisassembleInstr>,
    loc: u16,
    mark_halt: bool,
    rom: &ROM,
    emulator: &Arc<Emulation>,
) {
    let mark_break = emulator.has_breakpoint_at(loc);

    if mark_halt {
        ui.label(egui_phosphor::regular::PLAY);
    } else if mark_break {
        if ui
            .colored_label(Color32::RED, egui_phosphor::fill::CIRCLE)
            .clicked()
        {
            emulator.toggle_breakpoint(loc);
        }
    } else {
        if ui.label("       ").clicked() {
            emulator.toggle_breakpoint(loc);
        }
    }

    if ui.label(format!("0x{:X}      ", loc)).clicked() {
        emulator.toggle_breakpoint(loc);
    }

    let loc_u = loc as usize;
    let (instr_text, confirmed) = if let Some(instr) = may_instr {
        ui.label(byte_text(loc_u, instr.instr.len(), rom));

        let text = instr
            .instr
            .text(Some(&rom[loc_u..(loc_u + instr.instr.len())]));
        if instr.instr.op_code == psy::arch::sm83::INSTR_INVALID.op_code {
            (format!("{} op_code=0x{:x}", text, rom[loc_u]), false)
        } else {
            (text, instr.confirmed)
        }
    } else {
        ui.label(byte_text(loc_u, 1, rom));
        ("???".to_string(), false)
    };

    if confirmed {
        ui.label(instr_text);
    } else {
        ui.colored_label(COLOUR_UNCONFIRMED_INSTR, instr_text);
    }
    ui.end_row();
}

// Collect all confirmed instructions in the given memory range
fn instr_in_range(
    cache: &HashMap<u16, DisassembleInstr>,
    start: u16,
    end: u16,
) -> Vec<(u16, Option<&DisassembleInstr>)> {
    let mut result = Vec::with_capacity((end - start) as usize);

    let mut i = start;
    while i < end {
        let loc = i;
        let may_instr = cache.get(&loc);
        if let Some(instr) = may_instr {
            result.push((loc, Some(instr)));
            i += instr.instr.len() as u16;
        } else {
            result.push((loc, None));
            i += 1;
        }
    }

    result
}

fn byte_text(loc: usize, instr_len: usize, rom: &ROM) -> String {
    let txt = match instr_len {
        1 => format!("{:02X}      ", rom[loc]),
        2 => format!("{:02X} {:02X}   ", rom[loc], rom[loc + 1]),
        3 => format!("{:02X} {:02X} {:02X}", rom[loc], rom[loc + 1], rom[loc + 2]),
        0 | _ => "        ".to_string(),
    };
    format!("{}           ", txt)
}

// make sure that the instruction at the current pc is disassembled
// and in the disassembly cache
fn disassemble_pc(
    pc: u16,
    dmg: &DMG<Instant>,
    cache: &mut HashMap<u16, DisassembleInstr>,
    confirmed: bool,
) {
    let may_pc_instr = cache.get(&pc);

    if let Some(instr) = may_pc_instr
        && instr.confirmed
    {
        // do nothing and keep the confirmed instruction as is
    } else {
        let instr = psy::arch::sm83::decode(dmg.mc.read(pc).expect("instruction"));
        let dis = DisassembleInstr { confirmed, instr };
        cache.insert(pc, dis.clone());
    }
}

// pc must be in the pc_min, pc_max range
fn predict_disassemble_around_pc(
    pc_min: u16,
    pc_max: u16,
    dmg: &DMG<Instant>,
    cache: &mut HashMap<u16, DisassembleInstr>,
) {
    let mut pc = pc_min;
    while pc <= pc_max {
        let may_instr = cache.get(&pc);
        if let Some(instr) = may_instr {
            pc += instr.instr.len() as u16;
        } else {
            let instr = psy::arch::sm83::decode(dmg.mc.read(pc).expect("instruction"));

            // check that we do not "override" a confirmed instruction as this decoded
            // instruction might be a false positive one
            let mut overrides_confirmed = (false, 0);
            for i in 1..instr.len() {
                let may_instr = cache.get(&(pc + i as u16));
                if let Some(instr) = may_instr
                    && instr.confirmed
                {
                    overrides_confirmed = (true, pc + i as u16);
                }
            }

            if overrides_confirmed.0 {
                for i in 0..overrides_confirmed.1 {
                    // add invalid instructions, as something is wrong with the decode state
                    cache.insert(
                        pc + i as u16,
                        DisassembleInstr {
                            confirmed: false,
                            instr: &psy::arch::sm83::INSTR_INVALID,
                        },
                    );
                    // resync pc with the confirmed instrution and continue from there
                    pc = overrides_confirmed.1
                }
            } else {
                cache.insert(
                    pc,
                    DisassembleInstr {
                        confirmed: false,
                        instr,
                    },
                );
                pc += instr.len() as u16;
            }
        }
    }
}
