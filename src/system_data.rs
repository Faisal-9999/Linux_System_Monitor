//AFTER COMPLETING THIS WORK ON PER PROCESS CPU_USAGE in THE LOG PARSER FILE
//AND THEN WORK ON GET_TABLE_DATA in database connectivity file

use sysinfo::{System, Networks, Disks};
use std::{iter, thread, time::Duration};

pub struct SystemData {
    pub hardware : System,
    pub network : Networks,
    pub disks : Disks
}

struct Snapshot {
    overall_cpu_used : f32,
    overall_ram_used : u64,
    overall_net_down_kb : f64,
    overall_net_up_kb : f64,
    overall_disk_read_mb : f64,
    overall_disk_write_mb : f64,
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


        iter::from_fn(move || {
            thread::sleep(Duration::from_secs(1));

            self.hardware.refresh_cpu_all();
            self.hardware.refresh_memory();
            self.network.refresh(false);
            self.disks.refresh(false);

            let mut total_read_mb = 0.0;
            let mut total_write_mb = 0.0;

            for disk in self.disks.list() {
                total_read_mb += disk.usage().read_bytes as f64 / 1024.0 / 1024.0;
                total_write_mb += disk.usage().written_bytes as f64 / 1024.0 / 1024.0;
            }

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
                overall_disk_read_mb : total_read_mb,
                overall_disk_write_mb : total_write_mb
            })
        })
    }
}