use crate::view_models::MessageTableViewModel;
use eframe::egui::{Label, Sense, TopBottomPanel, Ui};
use egui_extras::{Column, TableBuilder};

pub(crate) struct MessageTableView<'a> {
    model: &'a mut MessageTableViewModel,
}

impl<'a> MessageTableView<'a> {
    pub(crate) fn new(model: &'a mut MessageTableViewModel) -> Self {
        Self { model }
    }

    pub(crate) fn view(&mut self, ui: &mut Ui) {
        ui.add_space(5.0);
        ui.heading(self.model.heading());

        TopBottomPanel::bottom("message_filter_panel").show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut changed = false;
                for type_filter in &mut self.model.message_types {
                    if ui
                        .selectable_label(type_filter.selected, type_filter.name)
                        .clicked()
                    {
                        type_filter.selected = !type_filter.selected;
                        changed = true;
                    }
                }
                if changed {
                    self.model.update_filter();
                }
            });
        });

        TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .auto_shrink(false)
            .sense(Sense::click())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Channel");
                });
                header.col(|ui| {
                    ui.strong("in/out");
                });
                header.col(|ui| {
                    ui.strong("Type");
                });
            })
            .body(|body| {
                body.rows(20.0, self.model.message_count(), |mut row| {
                    let Some(row_model) = self.model.get_row(row.index()) else {
                        return;
                    };
                    if row_model.is_selected {
                        row.set_selected(true);
                    }
                    row.col(|ui| {
                        ui.add(Label::new(row_model.channel_id).selectable(false));
                    });
                    row.col(|ui| {
                        ui.add(Label::new(row_model.direction).selectable(false));
                    });
                    row.col(|ui| {
                        ui.add(Label::new(row_model.message).selectable(false));
                    });
                    if row.response().clicked() {
                        self.model.select_message(row.index());
                    }
                })
            });
    }
}
