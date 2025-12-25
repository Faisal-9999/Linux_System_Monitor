//AFTER COMPLETING THIS WORK ON PER PROCESS CPU_USAGE in THE LOG PARSER FILE

use sysinfo::{System, Networks};
use std::{thread, time::Duration};
pub struct SystemData {
    hardware : System,
    network : Networks
}

impl SystemData {
    fn initialize() -> Self {
        Self {
            hardware : System::new_all(),
            network : Networks::new_with_refreshed_list(),
        }
    }

    fn refresh(&mut self) {
        self.hardware.refresh_cpu_all();
        self.hardware.refresh_memory();
        self.network.refresh(false);
    }

    fn cpu_usage(&self) -> f64 {
        //NEED TO WRITE CODE FOR THIS too tired rn
        //SAme for functions below but will change it later
        0.0
    }

    fn ram_usage(&self) -> f64 {
        0.0
    }

    fn network_usage(&self) -> f64 {
        0.0
    }
}