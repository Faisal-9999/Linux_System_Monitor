#[derive(Debug)]
pub struct ProcessData {
    pub pid : u32,
    pub ppid : u32,
    pub process_name : String,
    pub cpu_usage : f64,
    pub ram_usage : f64,
    pub state : String,
    pub threads_used : u32,
}