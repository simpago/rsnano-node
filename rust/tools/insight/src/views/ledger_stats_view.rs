use crate::view_models::LedgerStatsViewModel;
use eframe::egui::Ui;
use egui_extras::{Size, StripBuilder};

pub(crate) struct LedgerStatsView<'a>(LedgerStatsViewModel<'a>);

impl<'a> LedgerStatsView<'a> {
    pub fn new(model: LedgerStatsViewModel<'a>) -> Self {
        Self(model)
    }

    pub fn view(&self, ui: &mut Ui) {
        ui.label("Blocks");

        ui.label("bps:");
        StripBuilder::new(ui)
            .size(Size::exact(35.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.label(self.0.blocks_per_second());
                })
            });

        ui.label("cps:");
        StripBuilder::new(ui)
            .size(Size::exact(35.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.label(self.0.confirmations_per_second());
                })
            });

        ui.label("blocks:");
        ui.label(self.0.block_count());
        ui.add_space(10.0);
        ui.label("cemented:");
        ui.label(self.0.cemented_count());
    }
}
