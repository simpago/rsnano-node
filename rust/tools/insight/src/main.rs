mod channels;
mod ledger_stats;
mod message_collection;
mod message_rate_calculator;
mod message_recorder;
mod node_factory;
mod node_runner;
mod nullable_runtime;
mod rate_calculator;
mod view_models;
mod views;

use eframe::egui;
use tokio::runtime::Runtime;
use views::AppView;

fn main() -> eframe::Result {
    let runtime = Runtime::new().unwrap();
    let runtime_handle = runtime.handle().clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RsNano Insight",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_zoom_factor(1.15);
            Ok(Box::new(AppView::new(runtime_handle)))
        }),
    )
}
