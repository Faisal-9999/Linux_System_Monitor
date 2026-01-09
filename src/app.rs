use eframe::egui;
use crate::process_data::ProcessData;
use crate::database_connect::PostgresConnector;
use crate::log_parser;
use crate::system_data::{SystemData, Snapshot};
use std::time::Duration;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

pub struct LinuxApp {
    system_data: SystemData,
    db_connector: Option<PostgresConnector>,
    process_data: Vec<ProcessData>,

    cpu_per_core_history: Vec<Vec<f64>>,
    
    ram_history: Vec<f64>,
    net_down_history: Vec<f64>,
    net_up_history: Vec<f64>,
    disk_read_history: Vec<f64>,
    disk_write_history: Vec<f64>,
    
    last_disk_read: f64,
    last_disk_write: f64,
    last_net_down: f64,
    last_net_up: f64,
    cpu_process_usage: HashMap<u32, f64>,
    snapshot_rx: Receiver<Snapshot>,

    prev_disk_read_bytes: u64,
    prev_disk_write_bytes: u64,
    frame_counter: u32,
}

impl Default for LinuxApp {
    fn default() -> Self {
        let mut db = PostgresConnector::default();
        db.init_table();
        
        let system_data = SystemData::initialize();
        let num_cpus = system_data.hardware.cpus().len();

        let snapshot_rx = SystemData::start_snapshot_thread();

        let mut prev_disk_read_bytes: u64 = 0;
        let mut prev_disk_write_bytes: u64 = 0;
        for disk in system_data.disks.list() {
            prev_disk_read_bytes += disk.usage().read_bytes;
            prev_disk_write_bytes += disk.usage().written_bytes;
        }

        Self { 
            system_data,
            db_connector: Some(db),
            process_data: Vec::new(),
            
            cpu_per_core_history: vec![Vec::with_capacity(60); num_cpus],
            
            ram_history: Vec::with_capacity(60),
            net_down_history: Vec::with_capacity(60),
            net_up_history: Vec::with_capacity(60),
            disk_read_history: Vec::with_capacity(60),
            disk_write_history: Vec::with_capacity(60),
            
            last_disk_read: 0.0,
            last_disk_write: 0.0,
            last_net_down: 0.0,
            last_net_up: 0.0,
            cpu_process_usage: HashMap::new(),
            snapshot_rx,
            prev_disk_read_bytes,
            prev_disk_write_bytes,
            frame_counter: 0,
        }
    }
}

impl LinuxApp {
    pub fn start_app(&self) -> eframe::Result<()> {
        let app_view_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1400.0, 1000.0]),
            ..Default::default()
        };

        eframe::run_native(
            "Linux System Monitor", 
            app_view_options,
            Box::new(|_cc| Ok(Box::new(LinuxApp::default()))),
        )
    }

    fn update_system_metrics(&mut self) {
        self.frame_counter += 1;
        
        self.system_data.hardware.refresh_cpu_all();

        self.cpu_process_usage = log_parser::cpu_usage_calculator(&mut self.system_data.hardware);

        for (core_idx, cpu) in self.system_data.hardware.cpus().iter().enumerate() {
            if core_idx < self.cpu_per_core_history.len() {
                self.cpu_per_core_history[core_idx].push(cpu.cpu_usage() as f64);
                if self.cpu_per_core_history[core_idx].len() > 60 {
                    self.cpu_per_core_history[core_idx].remove(0);
                }
            }
        }

        let mut latest_snapshot: Option<Snapshot> = None;
        while let Ok(snap) = self.snapshot_rx.try_recv() {
            latest_snapshot = Some(snap);
        }

        if let Some(snap) = latest_snapshot {
            let ram_gb = snap.overall_ram_used as f64 / 1024.0;
            self.ram_history.push(ram_gb);
            if self.ram_history.len() > 60 {
                self.ram_history.remove(0);
            }

            self.last_net_down = snap.overall_net_down_kb;
            self.last_net_up = snap.overall_net_up_kb;
            self.net_down_history.push(self.last_net_down);
            self.net_up_history.push(self.last_net_up);
            if self.net_down_history.len() > 60 {
                self.net_down_history.remove(0);
                self.net_up_history.remove(0);
            }

            self.last_disk_read = snap.overall_disk_read_mb;
            self.last_disk_write = snap.overall_disk_write_mb;
            self.disk_read_history.push(self.last_disk_read);
            self.disk_write_history.push(self.last_disk_write);

            if self.disk_read_history.len() > 60 {
                self.disk_read_history.remove(0);
                self.disk_write_history.remove(0);
            }

        } else {
            self.system_data.hardware.refresh_memory();
            let ram_gb = self.system_data.hardware.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
            self.ram_history.push(ram_gb);
            if self.ram_history.len() > 60 {
                self.ram_history.remove(0);
            }

            let mut down_speed = 0.0;
            let mut up_speed = 0.0;
            for (_name, data) in self.system_data.network.iter() {
                down_speed += data.received() as f64 / 1024.0;
                up_speed += data.transmitted() as f64 / 1024.0;
            }
            self.last_net_down = down_speed;
            self.last_net_up = up_speed;
            self.net_down_history.push(down_speed);
            self.net_up_history.push(up_speed);
            if self.net_down_history.len() > 60 {
                self.net_down_history.remove(0);
                self.net_up_history.remove(0);
            }

            let mut total_read_bytes = 0u64;
            let mut total_write_bytes = 0u64;
            for disk in self.system_data.disks.list() {
                total_read_bytes += disk.usage().read_bytes;
                total_write_bytes += disk.usage().written_bytes;
            }
            
            let disk_read_speed_mb = ((total_read_bytes as f64 - self.prev_disk_read_bytes as f64) / 1024.0 / 1024.0).max(0.0);
            let disk_write_speed_mb = ((total_write_bytes as f64 - self.prev_disk_write_bytes as f64) / 1024.0 / 1024.0).max(0.0);
            
            self.prev_disk_read_bytes = total_read_bytes;
            self.prev_disk_write_bytes = total_write_bytes;
            
            self.last_disk_read = disk_read_speed_mb;
            self.last_disk_write = disk_write_speed_mb;
            self.disk_read_history.push(disk_read_speed_mb);
            self.disk_write_history.push(disk_write_speed_mb);
            if self.disk_read_history.len() > 60 {
                self.disk_read_history.remove(0);
                self.disk_write_history.remove(0);
            }
        }

        if self.frame_counter % 3 == 0 {
            if let Ok(pids) = log_parser::read_folder_names() {
                let mut collected_data = Vec::new();
                for pid_str in pids {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        let path = format!("/proc/{}/status", pid_str);
                        if let Ok(file) = std::fs::File::open(&path) {
                            if let Ok(process_data) = log_parser::process_data_definer(file) {
                                let mut process = process_data;
                                if let Some(&cpu_val) = self.cpu_process_usage.get(&pid) {
                                    process.cpu_usage = cpu_val;
                                }
                                collected_data.push(process);
                            }
                        }
                    }
                }

                if let Some(ref mut connector) = self.db_connector {
                    let _ = connector.save_data(collected_data);
                }
            }

            if let Some(ref mut connector) = self.db_connector {
                match connector.get_table_data() {
                    Ok(data) => self.process_data = data,
                    Err(_) => {}
                }
            }
        }
    }
}

impl eframe::App for LinuxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_system_metrics();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.heading("🐧 Linux System Monitor");
                    ui.separator();

                    self.draw_per_core_graph(ui);

                    ui.separator();

                    // ===== SYSTEM STATISTICS SECTION =====
                    self.draw_system_stats(ui);

                    // ===== GREY SEPARATOR LINE =====
                    ui.separator();

                    // ===== PROCESS DATA TABLE SECTION AT BOTTOM =====
                    self.draw_process_table(ui);
                });
        });

        // Request repaint at regular intervals (every 1 second)
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

// Helper methods for drawing UI sections
impl LinuxApp {
    fn draw_per_core_graph(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("📊 Per-Core CPU Performance");
            
            // Prepare lines for each CPU core
            let mut lines = Vec::new();
            let colors = [
                egui::Color32::RED,
                egui::Color32::BLUE,
                egui::Color32::GREEN,
                egui::Color32::YELLOW,
                egui::Color32::LIGHT_BLUE,
                egui::Color32::LIGHT_GREEN,
                egui::Color32::from_rgb(255, 165, 0), // Orange
                egui::Color32::from_rgb(255, 192, 203), // Pink
            ];

            for (core_idx, history) in self.cpu_per_core_history.iter().enumerate() {
                if !history.is_empty() {
                    let points: Vec<[f64; 2]> = history.iter()
                        .enumerate()
                        .map(|(i, &val)| [i as f64, val])
                        .collect();
                    
                    let color = colors[core_idx % colors.len()];
                    let line = egui_plot::Line::new(
                        format!("CPU {}", core_idx),
                        points
                    )
                    .stroke((2.0, color));
                    lines.push(line);
                }
            }

            // Draw plot with only first quadrant visible (y-axis from 0-100, x-axis from 0-60)
            egui_plot::Plot::new("cpu_core_plot")
                .width(500.0)
                .height(200.0)
                .include_x(0.0)
                .include_x(60.0)
                .include_y(0.0)
                .include_y(100.0)
                .set_margin_fraction(egui::vec2(0.0, 0.0))
                .x_axis_label("Time (samples)")
                .y_axis_label("CPU Utilization (%)")
                .legend(egui_plot::Legend::default())
                .show_grid(true)
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .show(ui, |plot_ui| {
                    for line in lines {
                        plot_ui.line(line);
                    }
                });
        });
    }

    fn draw_system_stats(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("⚙️ System Statistics");
            
            let grid_columns = 3;
            egui::Grid::new("system_stats_grid")
                .num_columns(grid_columns)
                .spacing([60.0, 20.0])
                .striped(true)
                .show(ui, |ui| {
                    // Row 1: CPU and RAM
                    ui.label("CPU Usage:");
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("{:.2}%", self.system_data.hardware.global_cpu_usage())
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();

                    ui.label("RAM Usage:");
                    let total_mem = self.system_data.hardware.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
                    let used_mem = self.system_data.hardware.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
                    ui.colored_label(
                        egui::Color32::BLUE,
                        format!("{:.2} GB / {:.2} GB", used_mem, total_mem)
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();

                    // Row 2: Disk read and write
                    ui.label("Disk Read:");
                    ui.colored_label(
                        egui::Color32::GREEN,
                        format!("{:.2} MB", self.last_disk_read)
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();

                    ui.label("Disk Write:");
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        format!("{:.2} MB", self.last_disk_write)
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();

                    // Row 3: Network stats
                    ui.label("Network Down:");
                    ui.colored_label(
                        egui::Color32::LIGHT_BLUE,
                        format!("{:.2} KB/s", self.last_net_down)
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();

                    ui.label("Network Up:");
                    ui.colored_label(
                        egui::Color32::LIGHT_GREEN,
                        format!("{:.2} KB/s", self.last_net_up)
                    );
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();
                });
        });
    }

    fn draw_process_table(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("📋 Per-Process Resource Usage");
            ui.separator();

            if !self.process_data.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        egui::Grid::new("process_table")
                            .num_columns(7)
                            .spacing([15.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                // Header row with styling
                                ui.colored_label(egui::Color32::WHITE, "PID");
                                ui.colored_label(egui::Color32::WHITE, "PPID");
                                ui.colored_label(egui::Color32::WHITE, "Process Name");
                                ui.colored_label(egui::Color32::WHITE, "CPU %");
                                ui.colored_label(egui::Color32::WHITE, "RAM (KB)");
                                ui.colored_label(egui::Color32::WHITE, "State");
                                ui.colored_label(egui::Color32::WHITE, "Threads");
                                ui.end_row();

                                // Data rows
                                for process in &self.process_data {
                                    ui.label(process.pid.to_string());
                                    ui.label(process.ppid.to_string());
                                    
                                    // Truncate long process names
                                    let display_name = if process.process_name.len() > 25 {
                                        format!("{}...", &process.process_name[..22])
                                    } else {
                                        process.process_name.clone()
                                    };
                                    ui.label(display_name);
                                    
                                    let cpu_color = if process.cpu_usage > 50.0 {
                                        egui::Color32::RED
                                    } else if process.cpu_usage > 25.0 {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::GREEN
                                    };
                                    ui.colored_label(cpu_color, format!("{:.2}", process.cpu_usage));
                                    
                                    ui.label(format!("{:.0}", process.ram_usage));
                                    ui.label(&process.state);
                                    ui.label(process.threads_used.to_string());
                                    ui.end_row();
                                }
                            });
                    });
            } else {
                ui.label("⚠️ No process data available. Make sure PostgreSQL is running with proper database setup.");
            }
        });
    }
}