
use hifitime::Epoch;
use sgp4::Elements;
use substring::Substring;
use std::{
    env,
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    str::FromStr,
};
use ureq::serde_json::{self};
use lazy_static::lazy_static;


lazy_static! {
    pub static ref ELEMENTS_STATIC: Vec<Elements> = load_all_elements();
}

pub fn get_satellites() -> anyhow::Result<()> {
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("satellites"); //directory for "satellites" folder
    if not_updated(&path) {
        let response = ureq::get("https://celestrak.com/NORAD/elements/gp.php")
            .query("GROUP", "active")
            .query("FORMAT", "json")
            .call()
            .expect("failed to access celestrak");
        let elements_vec: Vec<sgp4::Elements> = response.into_json()?;

        fs::create_dir_all(&path).expect("failed to craete satellites directory");
        let mut count = 0;
        let len = elements_vec.len();
        for elements in elements_vec {
            let id = elements.norad_id;
            let string = serde_json::to_string_pretty(&elements).unwrap();
            let string2 = string.substring(0, string.find("}").unwrap() + 1);
            let bytes = string2.as_bytes();
            let json_path = path.clone().join(format!("{}.json", id));
            //let mut file = File::write_all(json_path, string).expect(format!("error while creating json for satellite {}", id).as_str());
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(json_path)
                .expect(format!("error while creating json for satellite {}", id).as_str());
            file.set_len(0).expect("failed to delete contents of file");
            file.seek(SeekFrom::Start(0)).unwrap();
            file.write_all(bytes).unwrap();
            count += 1;
            print!("loading sats, {}/{}", count, len);
        }
        record_time(&path);
    }
    else {
        println!("tles not updated, updated too recently");
    }
    Ok(())
}

fn record_time(path: &PathBuf) {
    let now = Epoch::now().unwrap();
    let now_string = now.to_string();
    let now_bytes = now_string.as_bytes();
    let txt_path = path.clone().join("time.txt");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(txt_path)
        .expect("error while storing access time");
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(now_bytes).unwrap();
}

fn not_updated(path: &PathBuf) -> bool {
    let txt_path = path.clone().join("time.txt");
    let epoch_string = match fs::read_to_string(txt_path) {
        Ok(epoch) => epoch,
        Err(_e) => return true,
    };
    let last_epoch = Epoch::from_str(epoch_string.as_str()).unwrap();
    let now = Epoch::now().unwrap();
    let duration = now - last_epoch;
    if duration.to_seconds() > 3600.0 {
        return true;
    }
    return false;
}

pub fn get_elements_from_json(norad_id: u32) -> Result<Elements, String> {
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("satellites"); //directory for "satellites" folder
    let json_path = path.clone().join(format!("{}.json", norad_id));
    let elements_string_result = fs::read_to_string(&json_path);
    let elements: sgp4::Elements;
    match elements_string_result {
        Ok(elements_string) => elements = serde_json::from_str(elements_string.as_str().trim()).unwrap(),
        Err(_) => return Err("failed find satellite".into())
    }
    return Ok(elements);
}

pub fn load_all_elements() -> Vec<Elements>{

    let mut elements_vec = vec![];
    let mut path = env::current_exe().expect("error finding path to executable"); //finds path of executable
    path.pop(); //goes to parent directory
    path.push("satellites"); //directory for "satellites" folder

    let paths = fs::read_dir(path).expect("failed to read directory");
    for dir_result in paths {
        let dir = dir_result.unwrap();
        if format!("{:?}", dir.path()).contains(".json") {
            let elements_string = fs::read_to_string(dir.path()).expect(format!("failed to read json at {:?}", dir.path()).as_str());
            let elements: sgp4::Elements = serde_json::from_str(elements_string.as_str()).expect(format!("failed to read json in {:?}", dir.path()).as_str());
            elements_vec.push(elements);
        }

    }
    return elements_vec;
}
