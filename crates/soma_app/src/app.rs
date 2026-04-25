pub struct SomaApp {}

impl SomaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> SomaApp {
        SomaApp {}
    }
}

impl eframe::App for SomaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}
