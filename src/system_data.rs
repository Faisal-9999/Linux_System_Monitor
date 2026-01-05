use sysinfo::{System, Networks, Disks};
use std::{iter, thread, time::Duration, sync::mpsc::{self, Receiver}};

pub struct SystemData {
    pub hardware : System,
    pub network : Networks,
    pub disks : Disks
}

pub struct Snapshot {
    pub overall_cpu_used : f32,
    pub overall_ram_used : u64,
    pub overall_net_down_kb : f64,
    pub overall_net_up_kb : f64,
    pub overall_disk_read_mb : f64,
    pub overall_disk_write_mb : f64,
}

impl SystemData {
    pub fn initialize() -> Self {
        Self {
            hardware : System::new_all(),
            network : Networks::new_with_refreshed_list(),
            disks : Disks::new_with_refreshed_list(),
        }
    }

    fn system_usage_stream(&mut self) -> impl Iterator<Item = Snapshot> {
        self.hardware.refresh_all();
        self.network.refresh(false);
        self.disks.refresh(false);

        let mut prev_disk_read_bytes: u64 = 0;
        let mut prev_disk_write_bytes: u64 = 0;
        
        for disk in self.disks.list() {
            prev_disk_read_bytes += disk.usage().read_bytes;
            prev_disk_write_bytes += disk.usage().written_bytes;
        }

        iter::from_fn(move || {
            thread::sleep(Duration::from_secs(1));

            self.hardware.refresh_cpu_all();
            self.hardware.refresh_memory();
            self.network.refresh(false);
            self.disks.refresh(false);

            let mut current_disk_read_bytes: u64 = 0;
            let mut current_disk_write_bytes: u64 = 0;

            for disk in self.disks.list() {
                current_disk_read_bytes += disk.usage().read_bytes;
                current_disk_write_bytes += disk.usage().written_bytes;
            }

            let disk_read_speed_mb = ((current_disk_read_bytes as f64 - prev_disk_read_bytes as f64) / 1024.0 / 1024.0).max(0.0);
            let disk_write_speed_mb = ((current_disk_write_bytes as f64 - prev_disk_write_bytes as f64) / 1024.0 / 1024.0).max(0.0);

            prev_disk_read_bytes = current_disk_read_bytes;
            prev_disk_write_bytes = current_disk_write_bytes;

            let mut down_speed = 0.0;
            let mut up_speed = 0.0;

            for (_name, data) in self.network.iter() {
                down_speed += data.received() as f64 / 1024.0;
                up_speed += data.transmitted() as f64 / 1024.0;
            }

            Some(Snapshot{
                overall_cpu_used : self.hardware.global_cpu_usage(),
                overall_ram_used : self.hardware.used_memory() / 1024 / 1024,
                overall_net_down_kb : down_speed,
                overall_net_up_kb : up_speed,
                overall_disk_read_mb : disk_read_speed_mb,
                overall_disk_write_mb : disk_write_speed_mb
            })
        })
    }

    pub fn start_snapshot_thread() -> Receiver<Snapshot> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut sd = SystemData::initialize();
            let stream = sd.system_usage_stream();

            for snap in stream {
                if tx.send(snap).is_err() {
                    break;
                }
            }
        });

        rx
    }
}