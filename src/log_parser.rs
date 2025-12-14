use std::{fs, io, path::Path, path::PathBuf};
use std::fs::File;
use std::io::BufRead;
use crate::process_data::*;

const Location : &str = "/proc/";


pub fn read_folder_names() -> Result<Vec<String>, std::io::Error> {

    let entries = fs::read_dir(Location)?;

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

fn cpu_usage_calculator(pid : &mut u32) -> u32 {
    0
}
fn process_data_definer(file : File) -> io::Result<ProcessData> {

    let mut process_name = String::new();
    let mut pid : u32 = 0;
    let mut ppid : u32 = 0;
    let mut cpu_usage : u32 = 0;
    let mut threads_used : u32 = 0;
    let mut ram_usage : u32 = 0;
    let mut process_state : String = String::new();

    /*
        DATA THAT NEEDS TO BE PARSED FROM FILES ARE STATED ABOVE 
        SO FAR I THINK TENTATIVE I MIGHT ADD MORE LAYER ON ddeepending on
        how I feel about it GAY

        CALCULATING CPU MAY BE SOMEWHAT MORE CHALLENING Will look into it later on
        might need a math equation from it pretty sure Gemini showed the equation forgot to write down
    */

    let attributes : [AttributeAndValue; 6] = [
        AttributeAndValue::new(AttributeType::Name, String::from("Name")),
        AttributeAndValue::new(AttributeType::State, String::from("State")),
        AttributeAndValue::new(AttributeType::Pid, String::from("Pid")),
        AttributeAndValue::new(AttributeType::Ppid, String::from("Ppid")),
        AttributeAndValue::new(AttributeType::RamUsage, String::from("VmRSS")),
        AttributeAndValue::new(AttributeType::Threads, String::from("Threads")),
    ];



    let reader: io::BufReader<File> = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        let mut word : String = String::new();

        let mut attribute_value = String::new();

        let mut word_ended = false;

        let mut cleaned_val = String::new();

        for c in line.chars() {
            if c == ':' {
                word_ended = true;
                continue;
            }

            if !word_ended {
                word.push(c);
            }
            else {
                if c == ' ' {
                    continue;
                }
                else {
                    attribute_value.push(c);
                }
            }
        }

        //May uncomment later on if parser fucks up
        //println!("{} : {}", word, attribute_value); 

        let mut type_check: AttributeType = NONE;

        for i in &attributes {
            if i.att_name == word  {
                type_check = i.att_type;
                break;
            }
        }

        match type_check {
            Name => process_name = String::from(attribute_value.trim()),
            State => process_state = String::from(attribute_value.trim()), 
            Pid => pid = match attribute_value.trim().parse::<u32>() {
                Ok(val) => val,
                Err(e) => {
                    println!("pid {}", attribute_value);
                    panic!("{}", e)
                }
            },
            Ppid => {

                cleaned_val = u32_cleaner(attribute_value);
            
                ppid = match cleaned_val.trim().parse::<u32>() {
                    Ok(val) => val,
                    Err(e) => {
                        println!("ppid {}", cleaned_val);
                        panic!("{}", e)
                    }
                }
            },
            Threads =>  {

                cleaned_val = u32_cleaner(attribute_value);

                threads_used = match cleaned_val.trim().parse::<u32>() {
                    Ok(val) => val,
                    Err(e) => {
                    println!("threads {}", cleaned_val);
                    panic!("{}", e)
                    }
                }
            },
            RamUsage => {

                cleaned_val = u32_cleaner(attribute_value);
                
                ram_usage = match cleaned_val.trim().parse::<u32>() {
                    Ok(val) => val,
                    Err(e) => {
                        println!("ram {}", cleaned_val);
                        panic!("{}", e)
                    }
                }
            },
            NONE => ()
        }

        //TODO: FIX THE PARSING AND ASSIGNING ERRORS IN THIS 

    

    }

    println!("{} {} {} {} {} {}", pid, ppid, process_name, ram_usage, threads_used, process_state);

    Ok(ProcessData {
        pid,
        ppid,
        process_name,
        ram_usage,
        threads_used,
        cpu_usage,
        state : process_state
    })
}

//TODO: CHECK IF PROCESS DATA DEFINER IS WORKING PROPERLY AND SHOWING DATA
//BY DISPALAYING THE SHIT IN IT USING process_list_definer
//I guess I will do it tomorrow or some shit this kinda gay


pub fn process_list_definer(pids : &Vec<String>) -> io::Result<Vec<ProcessData>> {

    let mut process_table : Vec<ProcessData> = Vec::new();

    for pid in pids {

        let file = File::open(format!{"/proc/{}/status", pid})?;

        process_table.push(match process_data_definer(file) {
            Ok(data) => data,
            Err(_) => continue,
        });
    }

    Ok(process_table)
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