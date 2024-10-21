use crate::view_models::NodeRunnerViewModel;
use eframe::egui::{Button, Ui};

pub(crate) struct NodeRunnerView<'a> {
    model: &'a mut NodeRunnerViewModel,
}

impl<'a> NodeRunnerView<'a> {
    pub(crate) fn new(model: &'a mut NodeRunnerViewModel) -> Self {
        Self { model }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.start_beta_node_button(ui);
            self.start_live_node_button(ui);
            self.stop_button(ui);
            ui.label(self.model.status());
        });
    }

    fn start_live_node_button(&mut self, ui: &mut Ui) {
        if ui
            .add_enabled(self.model.can_start_node(), Button::new("Start live node"))
            .clicked()
        {
            self.model.start_live_node();
        }
    }

    fn start_beta_node_button(&mut self, ui: &mut Ui) {
        if ui
            .add_enabled(self.model.can_start_node(), Button::new("Start beta node"))
            .clicked()
        {
            self.model.start_beta_node();
        }
    }

    fn stop_button(&mut self, ui: &mut Ui) {
        if ui
            .add_enabled(self.model.can_stop_node(), Button::new("Stop node"))
            .clicked()
        {
            self.model.stop_node();
        }
    }
}
