pub struct process_data {
    pid : String,
    process_name : String,
    user : String,
    cpu_usage : u32,
    ram_usage : u32,
    avg_ram_usage : u32,
    avg_cpu_usage : u32
}

//TODO: BEFORE CONTINUING ANY FURTHER DECIDE PLAN AND SCOPE FOR PROJECT
//SHIT TOO WHACK RN
//DESIGN ARCHITECTURE OF THE PROJECT BEFORE STARTING WORK

impl process_data {
    pub fn new(pid : String, process_name : String, user : String, cpu_usage : u32, ram_usage : u32) -> process_data {
        process_data {
            pid, process_name, 
            user, cpu_usage, 
            ram_usage, 
            avg_ram_usage: 0, 
            avg_cpu_usage : 0
         }
    }
}