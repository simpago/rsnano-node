use crate::view_models::MessageViewModel;
use eframe::egui::{Grid, ScrollArea, Ui};

pub(crate) struct MessageView<'a> {
    model: &'a MessageViewModel,
}

impl<'a> MessageView<'a> {
    pub(crate) fn new(model: &'a MessageViewModel) -> Self {
        Self { model }
    }

    pub(crate) fn view(&self, ui: &mut Ui) {
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            Grid::new("details_grid").num_columns(2).show(ui, |ui| {
                ui.label("Date:");
                ui.label(self.model.date.clone());
                ui.end_row();

                ui.label("Channel:");
                ui.label(self.model.channel_id.clone());
                ui.end_row();

                ui.label("Direction:");
                ui.label(self.model.direction.clone());
                ui.end_row();

                ui.label("Type:");
                ui.label(self.model.message_type.clone());
                ui.end_row();
            });

            ui.add_space(20.0);
            ui.label(self.model.message.clone());
        });
    }
}
