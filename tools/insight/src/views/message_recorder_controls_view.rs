use crate::message_recorder::MessageRecorder;
use eframe::egui::Ui;

pub(crate) struct MessageRecorderControlsView<'a> {
    recorder: &'a MessageRecorder,
}

impl<'a> MessageRecorderControlsView<'a> {
    pub(crate) fn new(recorder: &'a MessageRecorder) -> Self {
        Self { recorder }
    }

    pub fn show(&self, ui: &mut Ui) {
        self.capture_check_box(ui);
        self.clear_button(ui);
    }

    fn capture_check_box(&self, ui: &mut Ui) {
        let mut checked = self.recorder.is_recording();
        ui.checkbox(&mut checked, "capture");
        if checked {
            self.recorder.start_recording()
        } else {
            self.recorder.stop_recording()
        }
    }

    fn clear_button(&self, ui: &mut Ui) {
        if ui.button("clear").clicked() {
            self.recorder.clear();
        }
    }
}
