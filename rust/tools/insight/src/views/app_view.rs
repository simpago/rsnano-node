use super::{
    LedgerStatsView, MessageRecorderControlsView, MessageStatsView, MessageTabView, NodeRunnerView,
    TabBarView,
};
use crate::view_models::{AppViewModel, Tab};
use eframe::egui::{self, global_theme_preference_switch, CentralPanel, TopBottomPanel};

pub(crate) struct AppView {
    model: AppViewModel,
}

impl AppView {
    pub(crate) fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let model = AppViewModel::with_runtime(runtime_handle);
        Self { model }
    }
}

impl AppView {
    fn show_node_runner(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("node_runner_panel").show(ctx, |ui| {
            ui.add_space(1.0);
            ui.horizontal(|ui| {
                NodeRunnerView::new(&mut self.model.node_runner).show(ui);
                ui.separator();
                MessageRecorderControlsView::new(&self.model.msg_recorder).show(ui);
            });
            ui.add_space(1.0);
        });
    }

    fn show_tabs(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            TabBarView::new(&mut self.model.tabs).show(ui);
        });
    }

    fn show_stats(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                global_theme_preference_switch(ui);
                ui.separator();
                MessageStatsView::new(self.model.message_stats()).view(ui);
                ui.separator();
                LedgerStatsView::new(self.model.ledger_stats()).view(ui);
            });
        });
    }
}

impl eframe::App for AppView {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.model.update();
        self.show_node_runner(ctx);
        self.show_tabs(ctx);
        self.show_stats(ctx);

        match self.model.tabs.selected_tab() {
            Tab::Messages => {
                MessageTabView::new(&mut self.model).show(ctx);
            }
            Tab::Peers => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("todo");
                });
            }
        }

        // Repaint to show the continuously increasing current block and message counters
        ctx.request_repaint();
    }
}
