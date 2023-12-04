// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

use crate::commands::*;
use crate::tracking::*;
mod commands;
mod rigctl;
mod tle;
mod tracking;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let splashscreen_window = app.get_window("splashscreen").unwrap();
            let main_window = app.get_window("main").unwrap();
            tauri::async_runtime::spawn(async move {
                // initialize your app here instead of sleeping :)
                println!("Initializing...");
                tle::get_satellites();
                println!("Done initializing.");

                splashscreen_window.close().unwrap();
                main_window.show().unwrap();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            listen,
            next,
            get_all_sat_x_y,
            get_all_r,
            calc_gmst_now,
            read_settings,
            write_settings,
            get_lat,
            get_lon,
            get_alt,
            get_sat_lat,
            get_sat_lon,
            get_name,
            get_launch_date
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    
}
