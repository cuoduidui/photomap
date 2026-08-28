mod db;
mod exif;
mod secure;
mod thumbnail;
mod geocode;
mod cluster;
mod coord;
mod export;
mod image_utils;
mod trip;
mod slideshow;
mod face;
mod commands;

use commands::AppState;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./data"));
            std::fs::create_dir_all(&app_data_dir).ok();

            let db_path = app_data_dir.join("photomap.db");
            let thumbs_dir = app_data_dir.join(".thumbnails");
            std::fs::create_dir_all(&thumbs_dir).ok();

            let db_path_str = db_path
                .to_str()
                .ok_or("应用数据目录包含非 UTF-8 字符，无法初始化数据库")?;
            let db_arc = std::sync::Arc::new(
                db::Database::open(db_path_str)
                    .map_err(|e| format!("打开数据库失败: {}", e))?,
            );

            let api_key = db_arc
                .get_config("amap_api_key")
                .ok()
                .flatten()
                .unwrap_or_default();

            let geocoder = geocode::Geocoder::new(&api_key, Some(db_arc.clone()));

            app.manage(AppState {
                db: db_arc,
                geocoder,
                data_dir: app_data_dir.to_string_lossy().to_string(),
                thumbs_dir: thumbs_dir.to_string_lossy().to_string(),
                cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_photos,
            commands::cancel_long_task,
            commands::get_all_photos,
            commands::get_photos_paginated,
            commands::count_photos,
            commands::get_photo_by_id,
            commands::delete_photo,
            commands::get_stats,
            commands::get_location_counts,
            commands::update_photo_location,
            commands::geocode_photo,
            commands::batch_geocode,
            commands::search_address,
            commands::get_custom_locations,
            commands::add_custom_location,
            commands::delete_custom_location,
            commands::get_config,
            commands::set_config,
            commands::get_map_bounds,
            commands::get_clustered_photos,
            commands::get_image_base64,
            commands::regenerate_thumbnails,
            commands::get_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::add_photo_tags,
            commands::remove_photo_tag,
            commands::get_photo_tags,
            commands::get_all_photo_tags,
            commands::save_export_file,
            commands::export_album_image,
            commands::export_album_video,
            commands::debug_log,
            commands::clear_debug_log,
            commands::auto_cluster_trips,
            commands::get_all_trips,
            commands::get_trip_photos,
            commands::save_journal,
            commands::delete_trip,
            commands::create_trip,
            commands::generate_ai_journal,
            commands::polish_ai_journal,
            commands::add_photos_to_trip,
            commands::remove_photos_from_trip,
            commands::get_photos_not_in_trip,
            commands::generate_slideshow,
            commands::open_file_location,
            commands::analyze_faces,
            commands::update_tag,
            commands::get_tag_photos,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
