use crate::view_models::TabBarViewModel;
use eframe::egui::Ui;

pub(crate) struct TabBarView<'a> {
    model: &'a mut TabBarViewModel,
}

impl<'a> TabBarView<'a> {
    pub(crate) fn new(model: &'a mut TabBarViewModel) -> Self {
        Self { model }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut selected = None;
            for tab in &self.model.tabs {
                if ui.selectable_label(tab.selected, tab.label).clicked() {
                    selected = Some(tab.value);
                }
            }
            if let Some(selected) = selected {
                self.model.select(selected);
            }
        });
    }
}
