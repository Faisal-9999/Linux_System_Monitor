mod app;
mod database_connect;
mod process_data;
mod log_parser;
mod system_data;

use crate::app::LinuxApp;

fn main() -> Result<(), eframe::Error> {
    let app = LinuxApp::default();
    app.start_app()
}