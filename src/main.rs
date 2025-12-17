mod app;
mod database_connect;
mod process_data;
mod log_parser;

use crate::database_connect::*;

fn main() {
    let mut a = PostgresConnector::default();
    a.init_table();
}
