use eframe::egui;

pub struct LinuxApp {
    
}

impl Default for LinuxApp {
    fn default() -> Self {
        Self { 

        }
    }
}

impl LinuxApp {
    pub fn start_app(&self) -> eframe::Result<()> {
        let app_view_options = eframe::NativeOptions {
            viewport : egui::ViewportBuilder::default()
                .with_inner_size([500.0, 600.0]),
                ..Default::default()
        };

        eframe::run_native("Linux Task Manager", 
        app_view_options,
         Box::new(|_cc| Ok(Box::new(LinuxApp::default()))),
        )
    }
}

impl eframe::App for LinuxApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Linux Task Manager");
        });
    }
}