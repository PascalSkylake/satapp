use std::{io::{Write, Read, Seek, SeekFrom}, env, fs::{self, OpenOptions, File}};
use hifitime::{Epoch, Duration, prelude::Formatter, efmt::consts::RFC2822};
use ureq::serde_json;

use crate::{tle::get_elements_from_json, rigctl, tracking, commands};
/*pub fn read_command() {
    //read input line into a string "s"
    let mut s = String::new();
    let _=stdout().flush();
    stdin().read_line(&mut s).expect("failed to read input");
    if let Some('\n')=s.chars().next_back() {
        s.pop();
    }
    if let Some('\r')=s.chars().next_back() {
        s.pop();
    }
    let first_space_index = s.find(' ').unwrap_or_else(|| s.len());
    println!("{}", s);
    let mut command = s.clone();
    command.truncate(first_space_index);
    match command.as_str() {
        "help" => help(),
        "next" => println!("{}", next(s)),
        "listen" => listen(s),
        "group" => group(s),
        _ => println!("command not recognized: {}", command.as_str())
    }
}*/

fn get_args(s: String) -> Vec<String> {
    let mut words: Vec<&str> = s.split_whitespace().collect();
    words.remove(0);
    return words.iter().map(|&s| s.to_owned()).collect();

}

fn help() {
    println!("commands:");
    println!("next [NORAD_ID] \t finds the next pass of the satellite with a given NORAD_ID");
    println!("listen [NORAD_ID] [FREQ_HZ] \t connects with rigctl server to automatically calculate doppler shift and listen at correct frequently");
}

#[tauri::command]
pub async fn next(id: &str) -> Result<Vec<String>, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            //this first part scans with minute resolution to find when the satellite is above the horizon
            let mut now = Epoch::now().unwrap();
            let elements_result = get_elements_from_json(idnum);
            let elements;
            let lat = commands::get_lat();
            let lon = commands::get_lon();
            match elements_result {
                Err(err) => return Err(err),
                Ok(result) => elements = result
            }
            let mut passes = vec![];
            let mut first_pass: Option<Epoch> = None;
            let mut num_mins: u32 = 0;
            for i in 0..10080 {
                let epoch = now + Duration::from_seconds(i as f64 * 60.0);
                match crate::tracking::get_elavation(6369555, lat, lon, &elements, epoch) {
                    Some(ele) => if ele > 0.0 {
                        num_mins += 1;
                        if first_pass.is_none() {
                            first_pass = Some(epoch);
                        }
                        
                        
                    }
                    _ => return Err("failed to find elevation of satellite".to_owned())
                        
                }
            }
            match first_pass {
                Some(epoch) => {
                    if num_mins > 8000 {
                        return Err("satellite is likely geostationary, a pass cannot be found".to_owned());
                    } else {
                        //find the first and last second of the pass
                        let mut first_sec = epoch.clone();
                        let mut last_sec = epoch.clone();
                        while crate::tracking::get_elavation(6369555, lat, lon, &elements, first_sec).unwrap() > 0.0 {
                            first_sec = first_sec - Duration::from_seconds(1.0);
                        }
                        while crate::tracking::get_elavation(6369555, lat, lon, &elements, last_sec).unwrap() > 0.0 {
                            last_sec = last_sec + Duration::from_seconds(1.0);
                        }
                        now = last_sec + Duration::from_seconds(10.0);
                        let fmt_first = Formatter::new(first_sec, RFC2822);
                        let fmt_last = Formatter::new(last_sec, RFC2822);
                        passes.push(format!("{fmt_first} to {fmt_last}"));
                    }
                }
                None => return Err("no pass was found within the next week".to_owned())
            }
            return Ok(passes);
            
        }
        Err(error) => return Err("epic fail, input was not a valid number".to_owned())
    }
}

#[tauri::command]
pub async fn next_ten(id: &str) -> Result<Vec<String>, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            //this first part scans with minute resolution to find when the satellite is above the horizon
            let mut now = Epoch::now().unwrap();
            let elements_result = get_elements_from_json(idnum);
            let elements;
            let lat = commands::get_lat();
            let lon = commands::get_lon();
            match elements_result {
                Err(err) => return Err(err),
                Ok(result) => elements = result
            }
            let mut passes = vec![];
            for j in 0..10 {
                let mut first_pass: Option<Epoch> = None;
                let mut num_mins: u32 = 0;
                for i in 0..10080 {
                    let epoch = now + Duration::from_seconds(i as f64 * 60.0);
                    match crate::tracking::get_elavation(6369555, lat, lon, &elements, epoch) {
                        Some(ele) => if ele > 0.0 {
                            num_mins += 1;
                            if first_pass.is_none() {
                                first_pass = Some(epoch);
                            }
                            
                            
                        }
                        _ => return Err("failed to find elevation of satellite".to_owned())
                            
                    }
                }
                match first_pass {
                    Some(epoch) => {
                        if num_mins > 8000 {
                            return Err("satellite is likely geostationary, a pass cannot be found".to_owned());
                        } else {
                            //find the first and last second of the pass
                            let mut first_sec = epoch.clone();
                            let mut last_sec = epoch.clone();
                            while crate::tracking::get_elavation(6369555, lat, lon, &elements, first_sec).unwrap() > 0.0 {
                                first_sec = first_sec - Duration::from_seconds(1.0);
                            }
                            while crate::tracking::get_elavation(6369555, lat, lon, &elements, last_sec).unwrap() > 0.0 {
                                last_sec = last_sec + Duration::from_seconds(1.0);
                            }
                            now = last_sec + Duration::from_seconds(10.0);
                            let fmt_first = Formatter::new(first_sec, RFC2822);
                            let fmt_last = Formatter::new(last_sec, RFC2822);
                            passes.push(format!("{fmt_first} to {fmt_last}"));
                        }
                    }
                    None => return Err("no pass was found within the next week".to_owned())
                }
            }
            return Ok(passes);
            
        }
        Err(error) => return Err("epic fail, input was not a valid number".to_owned())
    }
}

#[tauri::command]
pub async fn listen(id: &str, freq: &str) -> Result<String, String> {
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements_result = get_elements_from_json(idnum);
            let elements;
            match elements_result {
                Err(err) => return Err(err),
                Ok(result) => elements = result
            }
            match freq.parse::<u32>() {
                Ok(freqnum) => {
                    loop {
                        println!("{}", freqnum + (tracking::calc_doppler_shift(6369555, commands::get_lat(), commands::get_lon(), freqnum as f32, &elements)) as u32);
                        rigctl::set_frequency(freqnum + (tracking::calc_doppler_shift(6369555, commands::get_lat(), commands::get_lon(), freqnum as f32, &elements)) as u32);
                        //thread::sleep(time::Duration::from_millis(50));
                    }
                }
                Err(error) => return Err("epic fail, input was not a valid number".into())
            }
        }
        Err(error) => return Err("epic fail, input was not a valid number".into())
    }
}

//calls the correct group related function when the user inputs group command
fn group(s: String) {
    let args = get_args(s.clone());
    let arg = args.get(0).expect("no argument supplied");
    if arg.contains("new") {
        new_group(s.clone());
    }
    if arg.contains("delete") {
        delete_group(s.clone());
    }
    if arg.contains("list") {
        list_groups();
    }
    if arg.contains("remove") {
        remove_from_group(s.clone());
    }
    if arg.contains("add") {
        add_to_group(s.clone());
    }
}

fn new_group(s: String) {
    let args = get_args(s);
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    fs::create_dir_all(&path).expect("failed to craete groups");
    let name = args.get(1).expect("no name listed");
    let group_path = path.clone().join(format!("{}.txt", name));
    let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(group_path)
                .expect(format!("error while creating file for group {}", name).as_str());
    let mut clone = args.clone();
    let nums = clone.split_off(2);
    let json_string = serde_json::to_string_pretty(&nums).expect("failed to jsonify the string");
    file.write_all(json_string.as_bytes()).expect("failed to write file");
}

fn delete_group(s: String) {
    let args = get_args(s);
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    fs::create_dir_all(&path).expect("failed to create groups");
    let name = args.get(1).expect("no name listed");
    let group_path = path.clone().join(format!("{}.txt", name));
    fs::remove_file(group_path).expect("failed to delete group")
}

fn list_groups() {
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    fs::create_dir_all(&path).expect("failed to create groups");
    let groups = fs::read_dir(&path).expect("failed to read directory");

    let mut file_names = Vec::new();

    for group in groups {
        if let Ok(group) = group {
            if let Some(file_name) = group.file_name().to_str() {
                file_names.push(file_name.to_string());
            }
        }
    }

    println!("{:?}", file_names)
}

fn remove_from_group(s: String) {
    let args = get_args(s.clone());
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    fs::create_dir_all(&path).expect("failed to craete groups");
    let name = args.get(1).expect("no name listed");
    let group_path = path.clone().join(format!("{}.txt", name));
    
    let mut sats = get_group(name);
    args.iter().skip(2).for_each(|value| {
        sats = sats.clone().into_iter().filter(|x| x != value).collect();
    });
    delete_group(s.clone());
    let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(group_path)
                .expect(format!("error while creating file for group {}", name).as_str());
    let json_string = serde_json::to_string_pretty(&sats).expect("failed to jsonify the string");
    file.write_all(json_string.as_bytes()).expect("failed to write file");
}

fn add_to_group(s: String) {
    let args = get_args(s.clone());
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    fs::create_dir_all(&path).expect("failed to craete groups");
    let name = args.get(1).expect("no name listed");
    let group_path = path.clone().join(format!("{}.txt", name));
    
    let mut sats = get_group(name);
    args.iter().skip(2).for_each(|value| {
        if !sats.iter().any(|x| x == value) {
            sats.push(value.to_string());
        }
    });
    delete_group(s.clone());
    let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(group_path)
                .expect(format!("error while creating file for group {}", name).as_str());
    let json_string = serde_json::to_string_pretty(&sats).expect("failed to jsonify the string");
    file.write_all(json_string.as_bytes()).expect("failed to write file");
}

fn get_group(filename: &String) -> Vec<String> {
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("groups"); //directory for "groups" folder
    let group_path = path.clone().join(format!("{}.txt", filename));
    let mut file = File::open(group_path).expect("failed to create file");
    let mut json_string = String::new();
    file.read_to_string(&mut json_string);

    let storage: Vec<String> = serde_json::from_str(json_string.as_str()).expect("failed to read from string");
    return storage;
}

#[tauri::command]
pub fn write_settings(lat: String, lon: String) {
    let settings = vec![lat, lon];
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("settings"); //directory for "settings" folder
    fs::create_dir_all(&path).expect("failed to craete settings directory");
    let txt_path = path.clone().join("settings.txt");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(txt_path)
        .expect("error while accessing settings");
    let json_string = serde_json::to_string_pretty(&settings).expect("failed to jsonify the string");
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(json_string.as_bytes()).unwrap();
}

#[tauri::command]
pub fn read_settings() -> Vec<String> {
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("settings"); //directory for "settings" folder
    fs::create_dir_all(&path).expect("failed to craete settings directory");
    let path = path.clone().join("settings.txt");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path.clone())
        .expect("error while accessing settings");
    let mut file = File::open(path).expect("failed to create file");
    let mut json_string = String::new();
    file.read_to_string(&mut json_string);

    let storage = serde_json::from_str(json_string.as_str());
    if storage.is_ok() {
        return storage.unwrap();
    } else {
        return vec!["0".into(), "0".into()];
    }
    
}

#[tauri::command]
pub fn get_lat() -> f32{
    let lat = read_settings().get(0).unwrap().parse().unwrap();
    print!("\nsuccessfully read value {}", lat);
    return lat;
}

#[tauri::command]
pub fn get_lon() -> f32{
    let lon = read_settings().get(1).unwrap().parse().unwrap();
    print!("\nsuccessfully read value {}", lon);
    return lon;
}