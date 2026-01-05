use std::{fs, io};
use std::fs::File;
use std::io::BufRead;
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use crate::process_data::*;

const LOCATION : &str = "/proc/";


pub fn read_folder_names() -> Result<Vec<String>, std::io::Error> {

    let entries = fs::read_dir(LOCATION)?;

    let mut folder_names : Vec<String> = Vec::new();

    for entry in entries {
        let entry = entry?;

        if entry.metadata()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {

                let mut is_valid = true;

                for c in name.chars() {
                    if !c.is_ascii_digit() {
                        is_valid = false;
                        break;
                    }
                }

                if is_valid {
                    folder_names.push(name.to_owned());
                }
            }
        }
    }

    Ok(folder_names)
}

#[derive(Clone, Copy)]
enum AttributeType {
    Name,
    State,
    Pid,
    Ppid,
    Threads,
    RamUsage,
    NONE,
}

use AttributeType::*;
struct AttributeAndValue {
    att_type : AttributeType,
    att_name : String,
}

impl AttributeAndValue {
    fn new(att_type : AttributeType, att_name : String) -> AttributeAndValue {
        AttributeAndValue { att_type, att_name }
    }
}


//NEED TO WORK ON THIS AND MAKE IT FASTER

//I think its faster now not sure
//Gemini solution was 400 times slower due to calculating each pid cpu_usage at once
//Reduced O(N) time complexity to O(1) by calculating process cpu_usage at once and waiting a second
//outside the function by putting the main thread to sleep

pub fn cpu_usage_calculator(sys : &mut System) -> HashMap<u32, f64> {

    sys.refresh_cpu_usage();

    let cpu = ProcessRefreshKind::nothing().with_cpu();

    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, cpu);

    sys.processes()
        .iter()
        .filter(|(_, p) | p.cpu_usage() > 0.0)
        .map(|(pid, process)|
            (pid.as_u32(), process.cpu_usage() as f64)
        )
        .collect()
}

pub fn process_data_definer(file : File) -> io::Result<ProcessData> {

    let mut process_name: Option<String> = None;
    let mut pid : Option<u32> = None;
    let mut ppid: Option<u32> = None; 
    let mut threads_used : Option<u32> = None;
    let mut ram_usage : Option<f64> = None;
    let mut process_state : Option<String> = None;

    let attributes : [AttributeAndValue; 6] = 
    [
        AttributeAndValue::new(AttributeType::Name, String::from("Name")),
        AttributeAndValue::new(AttributeType::State, String::from("State")),
        AttributeAndValue::new(AttributeType::Pid, String::from("Pid")),
        AttributeAndValue::new(AttributeType::Ppid, String::from("PPid")),
        AttributeAndValue::new(AttributeType::RamUsage, String::from("VmRSS")),
        AttributeAndValue::new(AttributeType::Threads, String::from("Threads")),
    ];

    let reader: io::BufReader<File> = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let mut word : String = String::new();
        let mut attribute_value = String::new();
        let mut word_ended = false;

        for c in line.chars() {
            if c == ':' {
                word_ended = true;
                continue;
            }

            if !word_ended {

                if c != '\t' {
                    word.push(c);
                }
            }
            else {

                if c == ' ' || c == '\t' {
                    continue;
                }
                else {
                    attribute_value.push(c);
                }
            }
        }

        let mut type_check: AttributeType = NONE;

        for i in &attributes {
            if i.att_name == word  {
                type_check = i.att_type;
                break;
            }
        }

        match type_check {
            Name => process_name = Some(String::from(attribute_value.trim())),
            State => process_state = Some(String::from(attribute_value.trim())), 
            Pid => pid = attribute_value.trim().parse::<u32>().ok(),
            Ppid => {
                let cleaned_val = u32_cleaner(attribute_value);
                ppid = cleaned_val.parse::<u32>().ok();
            },
            Threads =>  {
                let cleaned_val = u32_cleaner(attribute_value);
                threads_used = cleaned_val.parse::<u32>().ok();
            },
            RamUsage => {
                let cleaned_val = u32_cleaner(attribute_value);
                ram_usage = cleaned_val.parse::<f64>().ok();
            },
            _ => ()
        }
    }

    if ram_usage.is_none() {
        ram_usage = Some(0.0);
    }

    if process_name.is_none() || pid.is_none() || ppid.is_none()
        || ram_usage.is_none() || process_state.is_none() || threads_used.is_none() {
            Err(io::Error::new(io::ErrorKind::NotFound, "Missing required field"))
    }
    else {


        Ok(ProcessData {
            pid: pid.unwrap(),
            ppid: ppid.unwrap(),
            process_name: process_name.unwrap(),
            ram_usage: ram_usage.unwrap(),
            threads_used: threads_used.unwrap(),
            cpu_usage : 0.0,
            state : process_state.unwrap()
        })
    }
}

fn u32_cleaner(var : String) -> String {
    let mut cleaned_val = String::new();

    for c in var.chars() {
        if c.is_ascii_digit() {
            cleaned_val.push(c);
        }   
    }

    cleaned_val
}