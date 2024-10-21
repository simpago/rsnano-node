use crate::view_models::MessageStatsViewModel;
use eframe::egui::Ui;
use egui_extras::{Size, StripBuilder};

pub(crate) struct MessageStatsView<'a>(MessageStatsViewModel<'a>);

impl<'a> MessageStatsView<'a> {
    pub fn new(model: MessageStatsViewModel<'a>) -> Self {
        Self(model)
    }

    pub fn view(&self, ui: &mut Ui) {
        ui.label("Messages");
        ui.label("out/s:");
        StripBuilder::new(ui)
            .size(Size::exact(35.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.label(self.0.send_rate());
                })
            });

        ui.label("in/s:");
        StripBuilder::new(ui)
            .size(Size::exact(35.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.label(self.0.receive_rate());
                })
            });
    }
}
