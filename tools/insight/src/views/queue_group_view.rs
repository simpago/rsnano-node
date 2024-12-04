use crate::view_models::QueueGroupViewModel;
use eframe::egui::{Align, Layout, ProgressBar, RichText, Ui};
use egui_extras::{Size, StripBuilder};

pub(crate) fn show_queue_group(ui: &mut Ui, model: QueueGroupViewModel) {
    ui.heading(model.heading);

    for queue in model.queues {
        ui.horizontal(|ui| {
            StripBuilder::new(ui)
                .size(Size::exact(100.0))
                .size(Size::exact(300.0))
                .size(Size::remainder())
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(queue.label);
                        });
                    });
                    strip.cell(|ui| {
                        let visuals = ui.visuals();
                        let (foreground, background) = if visuals.dark_mode {
                            queue.color.as_dark_colors()
                        } else {
                            queue.color.as_light_colors()
                        };
                        ui.add(
                            ProgressBar::new(queue.progress)
                                .text(RichText::new(queue.value).color(foreground))
                                .fill(background),
                        );
                    });
                    strip.cell(|ui| {
                        ui.label(queue.max);
                    });
                });
        });
    }
}
