use crate::process_data::ProcessData;

use postgres::{Client, NoTls};

pub struct PostgresConnector {
    client : Client,
}

impl Default for PostgresConnector {
    fn default() -> Self {
        PostgresConnector {
            client :  Client::connect("host=localhost user=postgres password=Fajita2231 dbname=db_project",
             NoTls).unwrap(),
        }
    }
}

impl PostgresConnector {
    pub fn save_data(&mut self, data : Vec<ProcessData>) {
        
    }

    pub fn init_table(&mut self) {
        self.client.batch_execute("CREATE TABLE IF NOT EXISTS Process_Table (
            pid INTEGER PRIMARY KEY,
            ppid INTEGER,
            name TEXT NOT NULL,
            cpu_usage REAL,
            ram_usage REAL,
            state TEXT,
            threads INTEGER
        )").unwrap();
    }

    pub fn check_table_exists(&mut self) -> Result<bool, postgres::Error> {
        Ok(false)
    }
}