use crate::process_data::ProcessData;

use postgres::{Client, NoTls};

pub struct PostgresConnector {
    client : Client,
}

impl Default for PostgresConnector {
    fn default() -> Self {
        PostgresConnector {
            client :  Client::connect("host=localhost user=faisal password=Fajita2231 dbname=db_project",
             NoTls).unwrap(),
        }
    }
}

impl PostgresConnector {
    pub fn save_data(&mut self, data : Vec<ProcessData>) -> Result<(), postgres::Error> {

        let mut transaction = self.client.transaction()?;

        for item in data {

            transaction.execute
            ("Insert INTO Process_Table (pid, ppid, name, cpu_usage, ram_usage, state, threads)
                        VALUES ($1, $2, $3, $4, $5, $6, $7)
                        ON CONFLICT (pid) DO UPDATE SET
                        cpu_usage = EXCLUDED.cpu_usage,
                        ram_usage = EXCLUDED.ram_usage,
                        state = EXCLUDED.state,
                        threads = EXCLUDED.threads",
                &[
                            &(item.pid as i32), 
                            &(item.ppid as i32), 
                            &item.process_name, 
                            &item.cpu_usage, 
                            &item.ram_usage, 
                            &item.state,
                            &(item.threads_used as i32) 
                        ]
            )?;
        }
        
        transaction.commit()?;
        

        Ok(())
    }

pub fn init_table(&mut self) {
        self.client.batch_execute("CREATE TABLE IF NOT EXISTS Process_Table (
            pid INTEGER PRIMARY KEY,
            ppid INTEGER,
            name TEXT NOT NULL,
            cpu_usage DOUBLE PRECISION, 
            ram_usage DOUBLE PRECISION,
            state TEXT,
            threads INTEGER
        )").unwrap();
    }

    pub fn get_table_data(&mut self) -> Result<Vec<ProcessData>, postgres::Error> {
        let rows = self.client.query(
            "SELECT pid, ppid, name, cpu_usage, ram_usage, state, threads FROM Process_Table",
            &[]
        )?;

        let mut process_list = Vec::new();

        for row in rows {
            let process = ProcessData {
                pid: row.get::<_, i32>(0) as u32,
                ppid: row.get::<_, i32>(1) as u32,
                process_name: row.get(2),
                cpu_usage: row.get(3),
                ram_usage: row.get(4),
                state: row.get(5),
                threads_used: row.get::<_, i32>(6) as u32,
            };
            process_list.push(process);
        }

        Ok(process_list)
    }
}