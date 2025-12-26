mod app;
mod database_connect;
mod process_data;
mod log_parser;
mod system_data;
mod task_handler;

use core::panic;

use crate::{database_connect::*, log_parser::{process_list_definer, read_folder_names}};

fn main() {


    let mut a = PostgresConnector::default();
    a.init_table();

    let folders = match read_folder_names() {
        Ok(val) => val,
        Err(e) => panic!("{}", e)
    };

    let b = process_list_definer(&folders);

    let b = match b {
        Ok(e) => e,
        Err(e) => panic!("{}", e)
    };


    a.save_data(b);
}