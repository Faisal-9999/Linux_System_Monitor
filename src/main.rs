mod app;
mod database_connect;
mod process_data;
mod log_parser;
mod system_data;
mod task_handler;

use core::panic;

use sysinfo::System;
use std::thread;
use std::time::Duration;

use crate::{database_connect::*, log_parser::{process_list_definer, read_folder_names}};


fn main() {
    let mut a = PostgresConnector::default();
    a.init_table();

    // 1. Keep the System object alive outside the loop
    let mut sys = System::new_all();
    
    // 2. We need a loop, otherwise the program finishes before 
    // it can ever calculate the "change" in CPU usage.
    loop {
        let folders = match read_folder_names() {
            Ok(val) => val,
            Err(e) => {
                eprintln!("Error reading /proc: {}", e);
                continue;
            }
        };

        // 3. Pass the persistent 'sys' into the definer
        // On the very first run, this will return 0. 
        // On every run after the sleep, it will return REAL numbers.
        let b = match process_list_definer(&folders, &mut sys) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error defining processes: {}", e);
                continue;
            }
        };

        // 4. Save the data to Postgres
        a.save_data(b);
        // 5. CRITICAL: This sleep provides the "Time" for the "Work/Time" calculation.
        // Without this, the next loop happens too fast to measure any CPU delta.
        thread::sleep(Duration::from_secs(1));
    }
}