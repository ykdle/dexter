#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod parser;
use std::{time::{Duration, Instant}, path::PathBuf};

use hashbrown::HashMap;
use parser::models::*;

use rusqlite::{Connection, params};
use tauri::{Manager, api::process::{Command, CommandEvent}, LogicalSize, Size, SystemTray, CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem, WindowBuilder, SystemTrayEvent, Window};
use window_vibrancy::apply_blur;

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show_logs = CustomMenuItem::new("show-logs".to_string(), "Show Logs");
    let show_meter = CustomMenuItem::new("show-meter".to_string(), "Show Meter");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide Meter");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show_logs)
        .add_item(show_meter)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .setup(|app| {
            let meter_window = app.get_window("main").unwrap();
            meter_window.set_always_on_top(true)
                .expect("failed to set windows always on top");
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
              meter_window.open_devtools();
            }

            meter_window.set_size(Size::Logical(LogicalSize { width: 500.0, height: 350.0 })).unwrap();

            #[cfg(target_os = "windows")]
            apply_blur(&meter_window, Some((10, 10, 10, 50))).expect("Unsupported platform! 'apply_blur' is only supported on Windows");
            let mut resource_path = app.path_resolver().resource_dir().expect("could not get resource dir");
            match setup_db(&mut resource_path) {
                Ok(_) => (),
                Err(e) => {
                    println!("error setting up database: {}", e);
                    meter_window.emit("error", Some(e))
                        .expect("failed to emit encounter-update");
                }
            }

            tauri::async_runtime::spawn(async move {
                let (mut rx, _child) = Command::new_sidecar("meter-core")
                    .expect("failed to start `meter-core` ")
                    .spawn()
                    .expect("Failed to spawn sidecar");
                // let (mut rx, _child) = Command::new_sidecar("loa-fake-log")
                //     .expect("failed to start `meter-core` ")
                //     .spawn()
                //     .expect("Failed to spawn sidecar");

                let mut encounter: Encounter = Default::default();
                let mut none: Option<Vec<Encounter>> = None;
                let mut last_time = Instant::now();
                let duration = Duration::from_millis(100);
                let mut reset = false;
                while let Some(event) = rx.recv().await {
                    if let CommandEvent::Stdout(line) = event {
                        parser::parse_line(Some(&meter_window), &mut none, &mut reset, &mut encounter, line);
                        let elapsed = last_time.elapsed();
                        if elapsed >= duration || reset {
                            let mut clone = encounter.clone();
                            let window = meter_window.clone();
                            tauri::async_runtime::spawn(async move {
                                if !clone.current_boss_name.is_empty() {
                                    clone.current_boss = clone.entities.get(&clone.current_boss_name).cloned();
                                    if clone.current_boss.is_none() {
                                        clone.current_boss_name = String::new();
                                    }
                                }
                                clone.entities.retain(|_, v| v.entity_type == EntityType::PLAYER && v.skill_stats.hits > 0);
                                if clone.entities.len() > 0 {
                                    // don't need to send these to the live meter
                                    clone.entities.values_mut()
                                        .for_each(|e| {
                                            e.damage_stats.dps_intervals = Vec::new();
                                        });
                                    window.emit("encounter-update", Some(clone))
                                        .expect("failed to emit encounter-update");
                                }
                            });
                        }
                        last_time = Instant::now();
                    }
                }
            });

            let logs_window = WindowBuilder::new(app, "logs", tauri::WindowUrl::App("/logs".into()))
                .title("LOA Logs")
                .min_inner_size(500.0, 300.0)
                .build()
                .expect("failed to create log window");
            logs_window.set_size(Size::Logical(LogicalSize { width: 800.0, height: 500.0 })).unwrap();
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                logs_window.open_devtools();
            }

            Ok(())
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if event.window().label() == "logs" {
                    event.window().hide().unwrap();
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                if let Some(meter) = app.get_window("main") {
                    meter.show().unwrap();
                    meter.unminimize().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    "hide" => {
                        if let Some(meter) = app.get_window("main") {
                            meter.hide().unwrap();
                        }
                    }
                    "show-meter" => {
                        if let Some(meter) = app.get_window("main") {
                            meter.show().unwrap();
                        }
                    }
                    "show-logs" => {
                        if let Some(logs) = app.get_window("logs") {
                            logs.show().unwrap();
                            logs.unminimize().unwrap();
                        } else {
                            WindowBuilder::new(app, "logs", tauri::WindowUrl::App("/logs".into()))
                                .title("LOA Logs")
                                .min_inner_size(500.0, 300.0)
                                .build()
                                .expect("failed to create log window");
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![load_encounters])
        .run(tauri::generate_context!())
        .expect("error while running application");
}


fn setup_db(resource_path: &mut PathBuf) -> Result<(), String> {
    resource_path.push("encounters.db");
    let conn = match Connection::open(resource_path) {
        Ok(conn) => conn,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    match conn.execute_batch("
        CREATE TABLE IF NOT EXISTS entity (
            name TEXT,
            encounter_id INTEGER NOT NULL,
            npc_id INTEGER,
            entity_type TEXT,
            class_id INTEGER,
            class TEXT,
            gear_score REAL,
            current_hp INTEGER,
            max_hp INTEGER,
            is_dead INTEGER,
            skills TEXT,
            damage_stats TEXT,
            skill_stats TEXT,
            PRIMARY KEY (name, encounter_id),
            FOREIGN KEY (encounter_id) REFERENCES encounter (id) ON DELETE CASCADE
        );
        CREATE INDEX encounter_fight_start_index
        ON encounter (fight_start desc);
        CREATE INDEX encounter_current_boss_index
        ON encounter (current_boss);
        ") {
        Ok(_) => (),
        Err(e) => {
            return Err(e.to_string());
        }
    }

    match conn.execute_batch("
        CREATE TABLE IF NOT EXISTS encounter (
            id INTEGER PRIMARY KEY,
            last_combat_packet INTEGER,
            fight_start INTEGER,
            local_player TEXT,
            current_boss TEXT,
            duration INTEGER,
            total_damage_dealt INTEGER,
            top_damage_dealt INTEGER,
            total_damage_taken INTEGER,
            top_damage_taken INTEGER,
            dps INTEGER,
            dps_intervals TEXT,
            buffs TEXT,
            debuffs TEXT
        );
        CREATE INDEX entity_encounter_id_index
        ON entity (encounter_id desc);
        CREATE INDEX entity_name_index
        ON entity (name);
        CREATE INDEX entity_class_index
        ON entity (class);
        ") {
        Ok(_) => (),
        Err(e) => {
            return Err(e.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
fn load_encounters(window: tauri::Window) -> Vec<Encounter> {
    let mut path = window.app_handle().path_resolver().resource_dir().expect("could not get resource dir");
    path.push("encounters.db");
    let conn = match Connection::open(path) {
        Ok(conn) => conn,
        Err(e) => {
            println!("cannot find path: {}", e);
            panic!("")
        }
    };

    let mut stmt = conn.prepare_cached("
        SELECT last_combat_packet, fight_start, local_player, current_boss, duration, total_damage_dealt, top_damage_dealt, total_damage_taken, top_damage_taken, dps, dps_intervals, buffs, debuffs
        FROM encounter
        ORDER BY fight_start DESC
        LIMIT 10
        ")
        .unwrap();
    let results = stmt.query_map(params![], |row| {
        
        let dps_intervals_str: Option<String> = row.get(10)?;
        let mut dps_intervals: Vec<(i32, i64)> = Vec::new();
        if dps_intervals_str.is_some() {
            let dps_intervals_str = dps_intervals_str.unwrap();
            // this will break most likely
            dps_intervals = serde_json::from_str(&dps_intervals_str).unwrap();
        }

        let buff_str: Option<String> = row.get(11)?;
        let mut buffs: HashMap<i32, StatusEffect> = HashMap::new();
        if buff_str.is_some() {
            let buff_str = buff_str.unwrap();
            buffs = serde_json::from_str(&buff_str).unwrap();
        }

        let debuff_str: Option<String> = row.get(12)?;
        let mut debuffs: HashMap<i32, StatusEffect> = HashMap::new();
        if debuff_str.is_some() {
            let debuff_str = debuff_str.unwrap();
            debuffs = serde_json::from_str(&debuff_str).unwrap();
        }

        Ok(Encounter {
            last_combat_packet: row.get(0)?,
            fight_start: row.get(1)?,
            local_player: row.get(2)?,
            current_boss_name: row.get(3)?,
            duration: row.get(4)?,
            encounter_damage_stats: EncounterDamageStats {
                total_damage_dealt: row.get(5)?,
                top_damage_dealt: row.get(6)?,
                total_damage_taken: row.get(7)?,
                top_damage_taken: row.get(8)?,
                dps: row.get(9)?,
                dps_intervals,
                buffs,
                debuffs,
                ..Default::default()
            },
            ..Default::default()
        })
    }).unwrap();

    let mut encounters: Vec<Encounter> = Vec::new();
    for encounter in results {
        encounters.push(encounter.unwrap());
    }

    encounters
}
