use crate::db::{self, CustomLocation, LocationCount, Photo, PhotoStats, Trip, PhotoCoord};
use crate::exif;
use crate::thumbnail;
use crate::geocode::{Geocoder, SearchResult};
use crate::cluster::{self, BoundingBox, ClusterPoint};
use crate::face;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};

/// 导入临时文件计数器（用于生成并发安全的临时文件名）
static IMPORT_TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct AppState {
    pub db: std::sync::Arc<crate::db::Database>,
    pub geocoder: Geocoder,
    pub data_dir: String,
    pub thumbs_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub total: usize,
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationRequest {
    pub photo_ids: Vec<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
}

fn scan_directory(dir: &str, visited: &mut std::collections::HashSet<std::path::PathBuf>) -> Vec<String> {
    let mut results = Vec::new();
    // 通过规范化路径防止符号链接导致的无限递归
    if let Ok(canonical) = std::fs::canonicalize(dir) {
        if !visited.insert(canonical) {
            return results;
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(scan_directory(path.to_string_lossy().as_ref(), visited));
            } else if let Some(path_str) = path.to_str() {
                if exif::is_supported_image(path_str) {
                    results.push(path_str.to_string());
                }
            }
        }
    }
    results
}

/// 将照片复制到应用数据目录，返回复制后的路径
/// 使用照片内容的 MD5 作为文件名：同一张照片（内容相同）无论从哪个路径导入都会去重，
/// 内容变化后也会自然生成新的文件，避免旧缓存失效的问题。
fn copy_photo_to_storage(src_path: &str, photos_dir: &str) -> Option<String> {
    use md5::{Md5, Digest};

    let src = Path::new(src_path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();

    // 先写入同目录下的临时文件，边复制边计算内容哈希，避免并发导入相同内容时互相覆盖
    let temp_path = Path::new(photos_dir).join(format!(
        ".importing_{}_{}",
        std::process::id(),
        IMPORT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let mut src_file = match std::fs::File::open(src) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("打开照片失败 {}: {}", src_path, e);
            return None;
        }
    };
    let mut tmp_file = match std::fs::File::create(&temp_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("创建临时文件失败 {}: {}", src_path, e);
            return None;
        }
    };

    let mut hasher = Md5::new();
    let mut buffer = [0u8; 128 * 1024];
    let copy_result = loop {
        match src_file.read(&mut buffer) {
            Ok(0) => break Ok(()),
            Ok(n) => {
                hasher.update(&buffer[..n]);
                if let Err(e) = tmp_file.write_all(&buffer[..n]) {
                    break Err(e);
                }
            }
            Err(e) => break Err(e),
        }
    };
    drop(tmp_file);

    if let Err(e) = copy_result {
        let _ = std::fs::remove_file(&temp_path);
        eprintln!("复制照片失败 {}: {}", src_path, e);
        return None;
    }

    let hash = format!("{:x}", hasher.finalize());

    let dest_path = format!("{}/{}.{}", photos_dir, hash, ext);

    if Path::new(&dest_path).exists() {
        let _ = std::fs::remove_file(&temp_path);
        return Some(dest_path);
    }

    match std::fs::rename(&temp_path, &dest_path) {
        Ok(_) => Some(dest_path),
        Err(e) => {
            // 并发导入相同内容时目标可能已被其他线程创建，此时视为成功
            if Path::new(&dest_path).exists() {
                let _ = std::fs::remove_file(&temp_path);
                return Some(dest_path);
            }
            let _ = std::fs::remove_file(&temp_path);
            eprintln!("复制照片失败 {}: {}", src_path, e);
            None
        }
    }
}

#[tauri::command]
pub async fn import_photos(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    is_folder: bool,
) -> Result<ImportResult, String> {
    let thumbs_dir = state.thumbs_dir.clone();
    std::fs::create_dir_all(&thumbs_dir).map_err(|e| e.to_string())?;

    // 创建照片存储目录
    let photos_dir = format!("{}/photos", state.data_dir);
    std::fs::create_dir_all(&photos_dir).map_err(|e| e.to_string())?;

    let all_files = if is_folder {
        let mut files = Vec::new();
        let mut visited = std::collections::HashSet::new();
        for p in &paths {
            files.extend(scan_directory(p, &mut visited));
        }
        files
    } else {
        paths.into_iter().filter(|p| exif::is_supported_image(p)).collect()
    };

    let total = all_files.len();
    if total == 0 {
        return Ok(ImportResult { total: 0, success: 0, skipped: 0, failed: 0 });
    }

    let db = state.db.clone();
    let app_arc = Arc::new(app);
    let processed = Arc::new(AtomicUsize::new(0));

    let mut success = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    // 分批次处理，控制内存占用 - 每批 100 张
    let batch_size = 100;
    let num_batches = (total + batch_size - 1) / batch_size;

    // 配置 rayon 线程池限制并发，防止系统资源耗尽
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().min(4)) // 最多 4 个线程
        .build()
        .unwrap();

    for batch_idx in 0..num_batches {
        let batch_start = batch_idx * batch_size;
        let batch_end = (batch_start + batch_size).min(total);
        let batch_files = &all_files[batch_start..batch_end];

        // 处理当前批次
        let batch_photos: Vec<Photo> = thread_pool.install(|| {
            batch_files
                .par_iter()
                .map(|file_path| {
                    let file_name = Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let file_size = std::fs::metadata(file_path).ok().map(|m| m.len() as i64);

                    // 复制照片到应用数据目录，保留原始文件夹不动
                    let stored_path = copy_photo_to_storage(file_path, &photos_dir);
                    let actual_path = stored_path.as_deref().unwrap_or(file_path);

                    let mut photo = db::new_photo(actual_path, &file_name, file_size);

                    // 解析 EXIF（从原始文件解析）
                    if let Some(exif_info) = exif::parse_exif(file_path) {
                        photo.taken_time = exif_info.taken_time;
                        photo.latitude = exif_info.latitude;
                        photo.longitude = exif_info.longitude;
                        photo.camera_model = exif_info.camera_model;
                        photo.exif_data = exif_info.raw_json;
                    }

                    // 生成缩略图（从复制的文件生成）
                    if let Ok(thumb_path) = thumbnail::generate_thumbnail(actual_path, &thumbs_dir) {
                        photo.thumbnail_path = Some(thumb_path);
                    }

                    // 更新进度
                    let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    if count % 10 == 0 || count == total {
                        let _ = app_arc.emit("import-progress", (count, total));
                    }

                    photo
                })
                .collect()
        });

        // 批量插入数据库
        match db.insert_photos_batch(&batch_photos) {
            Ok((s, sk)) => {
                success += s;
                skipped += sk;
            }
            Err(e) => {
                failed += batch_photos.len();
                eprintln!("Batch insert error (batch {}): {}", batch_idx, e);
            }
        }

        // 主动释放内存
        drop(batch_photos);
    }

    Ok(ImportResult {
        total,
        success,
        skipped,
        failed,
    })
}

#[tauri::command]
pub async fn get_all_photos(state: State<'_, AppState>) -> Result<Vec<Photo>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_all_photos().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_photos_paginated(limit: i64, offset: i64, state: State<'_, AppState>) -> Result<Vec<Photo>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_photos_paginated(limit, offset).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn count_photos(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.count_photos().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_photo_by_id(id: String, state: State<'_, AppState>) -> Result<Option<Photo>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_photo_by_id(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_photo(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    let paths = tokio::task::spawn_blocking(move || db.delete_photo(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;

    let thumbs_dir = state.thumbs_dir.clone();
    let photos_dir = format!("{}/photos", state.data_dir);
    for path in paths {
        let Some(path) = path else { continue };
        // 删除应用内存储的照片副本（仅当位于 data_dir/photos 下，避免误删用户原图）
        if path.starts_with(&photos_dir) {
            let _ = std::fs::remove_file(&path);
        }
        // 删除缩略图与人脸缩略图（仅限应用缩略图目录）
        if path.starts_with(&thumbs_dir) {
            let _ = std::fs::remove_file(&path);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<PhotoStats, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_stats().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_location_counts(state: State<'_, AppState>) -> Result<Vec<LocationCount>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_location_counts().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_photo_location(
    request: UpdateLocationRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    for id in &request.photo_ids {
        state
            .db
            .update_photo_location(
                id,
                request.latitude,
                request.longitude,
                request.province.as_deref(),
                request.city.as_deref(),
                request.district.as_deref(),
                request.address.as_deref(),
                true,
            )
            .map_err(|e| e.to_string())?;
    }

    if state.geocoder.has_api_key() && request.address.is_none() {
        if let Ok(result) = state.geocoder.reverse_geocode(request.latitude, request.longitude).await {
            for id in &request.photo_ids {
                let _ = state.db.update_photo_location(
                    id,
                    request.latitude,
                    request.longitude,
                    result.province.as_deref(),
                    result.city.as_deref(),
                    result.district.as_deref(),
                    result.address.as_deref(),
                    true,
                );
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn geocode_photo(
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !state.geocoder.has_api_key() {
        return Err("未配置高德API Key".to_string());
    }

    let photo = state.db.get_photo_by_id(&photo_id).map_err(|e| e.to_string())?
        .ok_or_else(|| "照片不存在".to_string())?;

    let (lat, lng) = match photo.latitude.zip(photo.longitude) {
        Some(v) => v,
        None => return Err("照片无GPS坐标".to_string()),
    };

    // 如果已经有省市区信息，跳过
    if photo.province.is_some() && photo.city.is_some() {
        return Ok(());
    }

    let result = state.geocoder.reverse_geocode(lat, lng).await
        .map_err(|e| e.to_string())?;

    // 优先保存高德返回的完整中文地址，解析不到时回退到街道/区县
    let address = result.address.clone().or_else(|| result.street_label());
    state.db.update_photo_location(
        &photo_id,
        lat,
        lng,
        result.province.as_deref(),
        result.city.as_deref(),
        result.district.as_deref(),
        address.as_deref(),
        false,
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn batch_geocode(
    app: AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<usize, String> {
    if !state.geocoder.has_api_key() {
        return Err("未配置高德API Key".to_string());
    }

    // 默认增量模式：只解析缺少省/市/地址的照片；force=true 时刷新全部
    let photos = state.db.get_photos_for_geocode(force.unwrap_or(false)).map_err(|e| e.to_string())?;
    let total = photos.len();
    if total == 0 {
        return Ok(0);
    }

    let coords: Vec<(f64, f64)> = photos
        .iter()
        .filter_map(|p| p.latitude.zip(p.longitude))
        .collect();

    let geocoder = &state.geocoder;
    let results = geocoder.batch_reverse_geocode(coords, |done, total| {
        let _ = app.emit("geocode-progress", (done, total));
    }).await;

    let mut updated = 0;
    let photo_coords: Vec<(String, f64, f64)> = photos
        .iter()
        .filter_map(|p| {
            p.latitude.and_then(|lat| p.longitude.map(|lng| (p.id.clone(), lat, lng)))
        })
        .collect();

    for (i, (lat, lng, result)) in results.into_iter().enumerate() {
        if let Some(geocode_result) = result {
            if i < photo_coords.len() {
                let (photo_id, _, _) = &photo_coords[i];
                let address = geocode_result
                    .address
                    .clone()
                    .or_else(|| geocode_result.street_label());
                let _ = state.db.update_photo_location(
                    photo_id,
                    lat,
                    lng,
                    geocode_result.province.as_deref(),
                    geocode_result.city.as_deref(),
                    geocode_result.district.as_deref(),
                    address.as_deref(),
                    false,
                );
                updated += 1;
            }
        }
    }

    Ok(updated)
}

#[tauri::command]
pub async fn search_address(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    state
        .geocoder
        .search_address(&keyword)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_custom_locations(state: State<'_, AppState>) -> Result<Vec<CustomLocation>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_custom_locations().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_custom_location(
    name: String,
    latitude: f64,
    longitude: f64,
    address: Option<String>,
    state: State<'_, AppState>,
) -> Result<CustomLocation, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.add_custom_location(&name, latitude, longitude, address.as_deref()).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_custom_location(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_custom_location(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

// --- Tags ---
#[tauri::command]
pub async fn get_tags(state: State<'_, AppState>) -> Result<Vec<db::Tag>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_tags().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_tag(name: String, tag_type: String, color: String, state: State<'_, AppState>) -> Result<db::Tag, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.create_tag(&name, &tag_type, &color).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_tag(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_tag(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_photo_tags(photo_id: String, tag_ids: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.add_photo_tags(&photo_id, &tag_ids).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_photo_tag(photo_id: String, tag_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.remove_photo_tag(&photo_id, &tag_id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_photo_tags(photo_id: String, state: State<'_, AppState>) -> Result<Vec<db::Tag>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_photo_tags(&photo_id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_all_photo_tags(state: State<'_, AppState>) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_all_photo_tags().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_config(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_config(&key).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_config(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    let key_c = key.clone();
    let value_c = value.clone();
    tokio::task::spawn_blocking(move || db.set_config(&key_c, &value_c).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;

    // 高德 Key 热更新：无需重启应用
    if key == "amap_api_key" {
        state.geocoder.set_api_key(&value);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_map_bounds(state: State<'_, AppState>) -> Result<Option<BoundingBox>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let coords = db.get_photo_coords().map_err(|e| e.to_string())?;
        let bounds = cluster::calculate_bounds(&coords).ok_or_else(|| "暂无已定位照片".to_string())?;
        // EXIF 坐标为 WGS-84，地图使用 GCJ-02，显示前转换
        let (min_lat, min_lng) = crate::coord::wgs84_to_gcj02(bounds.min_lat, bounds.min_lng);
        let (max_lat, max_lng) = crate::coord::wgs84_to_gcj02(bounds.max_lat, bounds.max_lng);
        Ok(Some(cluster::BoundingBox {
            min_lat,
            max_lat,
            min_lng,
            max_lng,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_clustered_photos(
    zoom: u8,
    state: State<'_, AppState>,
) -> Result<Vec<ClusterPoint>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let coords = db.get_photo_coords().map_err(|e| e.to_string())?;
        let points = cluster::cluster_photos(&coords, zoom, None);
        // 转换为 GCJ-02 供高德地图显示
        Ok(points
            .into_iter()
            .map(|mut p| {
                let (lat, lng) = crate::coord::wgs84_to_gcj02(p.latitude, p.longitude);
                p.latitude = lat;
                p.longitude = lng;
                p
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_image_base64(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let data_dir = state.data_dir.clone();
    let thumbs_dir = state.thumbs_dir.clone();
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let path = Path::new(&path);
        if !path.is_file() {
            return Err("文件不存在".to_string());
        }

        // 限制单张图片大小，避免一次性读取超大文件导致内存暴涨
        const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;
        let metadata = std::fs::metadata(path).map_err(|e| format!("读取文件信息失败: {}", e))?;
        if metadata.len() > MAX_IMAGE_BYTES {
            return Err(format!(
                "图片过大（约 {} MB），超过 {} MB 上限",
                metadata.len() / (1024 * 1024),
                MAX_IMAGE_BYTES / (1024 * 1024)
            ));
        }

        // 只允许访问应用数据目录、缩略图目录，或数据库中已登记的照片路径
        let canonical = path.canonicalize().map_err(|e| format!("无法解析路径: {}", e))?;
        let data_dir = Path::new(&data_dir).canonicalize().map_err(|e| e.to_string())?;
        let thumbs_dir = Path::new(&thumbs_dir).canonicalize().map_err(|e| e.to_string())?;
        let allowed = canonical.starts_with(&data_dir) || canonical.starts_with(&thumbs_dir);
        if !allowed {
            let known = db
                .photo_path_exists(&canonical)
                .map_err(|e| format!("查询照片失败: {}", e))?;
            if !known {
                return Err("无权访问该路径".to_string());
            }
        }

        let mut file = std::fs::File::open(&canonical).map_err(|e| e.to_string())?;
        let mut buffer = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

        let ext = canonical
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();

        let mime = match ext.as_str() {
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "svg" => "image/svg+xml",
            _ => "image/jpeg",
        };

        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buffer);
        Ok(format!("data:{};base64,{}", mime, b64))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn save_export_file(path: String, data_base64: String) -> Result<(), String> {
    use base64::Engine;
    let buffer = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim_start_matches("data:").split(',').last().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, &buffer).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn export_album_image(
    app: AppHandle,
    album_name: String,
    date_range: String,
    location: String,
    photo_paths: Vec<String>,
    output_path: String,
) -> Result<String, String> {
    crate::export::generate_collage(
        &app, &album_name, &date_range, &location, &photo_paths, &output_path,
    )
}

#[tauri::command]
pub fn export_album_video(
    app: AppHandle,
    album_name: String,
    date_range: String,
    photo_paths: Vec<String>,
    output_path: String,
) -> Result<String, String> {
    crate::export::generate_video(
        &app, &album_name, &date_range, &photo_paths, &output_path,
    )
}

#[tauri::command]
pub async fn debug_log(message: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let log_path = std::env::temp_dir().join("photomap_debug.log");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| e.to_string())?;
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        writeln!(file, "[{}] {}", timestamp, message).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clear_debug_log() -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let log_path = std::env::temp_dir().join("photomap_debug.log");
        std::fs::write(&log_path, "").map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn auto_cluster_trips(state: State<'_, AppState>) -> Result<Vec<Trip>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        crate::trip::run_clustering(&db)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_all_trips(state: State<'_, AppState>) -> Result<Vec<Trip>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.get_all_trips().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_trip_photos(trip_id: String, state: State<'_, AppState>) -> Result<Vec<Photo>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.get_trip_photos(&trip_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_photos_to_trip(
    trip_id: String,
    photo_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.add_photos_to_trip(&trip_id, &photo_ids).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_photos_from_trip(
    trip_id: String,
    photo_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.remove_photos_from_trip(&trip_id, &photo_ids).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_photos_not_in_trip(
    trip_id: String,
    page: i64,
    page_size: i64,
    state: State<'_, AppState>,
) -> Result<(Vec<Photo>, i64), String> {
    let db = state.db.clone();
    let db2 = db.clone();
    let trip_id_clone = trip_id.clone();
    let trip_id_clone2 = trip_id.clone();
    let offset = (page - 1) * page_size;

    let photos = tokio::task::spawn_blocking(move || {
        db.get_photos_not_in_trip(&trip_id_clone, page_size, offset).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let total = tokio::task::spawn_blocking(move || {
        db2.count_photos_not_in_trip(&trip_id_clone2).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok((photos, total))
}

#[tauri::command]
pub async fn save_journal(trip_id: String, text: String, journal_type: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.update_trip_journal(&trip_id, &text, &journal_type).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_trip(trip_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.delete_trip(&trip_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_trip(
    title: String,
    start_date: Option<String>,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<Trip, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.create_trip(&title, start_date.as_deref(), end_date.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn generate_ai_journal(
    app: AppHandle,
    trip_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.clone();

    let provider = db.get_config("ai_provider").unwrap_or(None).unwrap_or_else(|| "openai".to_string());
    let api_key = db.get_config("ai_api_key").unwrap_or(None).unwrap_or_default();
    let model = db.get_config("ai_model").unwrap_or(None).unwrap_or_else(|| "gpt-4o-mini".to_string());
    let base_url = db.get_config("ai_base_url").unwrap_or(None).unwrap_or_default();

    let db2 = state.db.clone();
    let trip_id2 = trip_id.clone();
    let (trip, photos, photo_tags_map) = tokio::task::spawn_blocking(move || {
        let trip = db2.get_all_trips()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|t| t.id == trip_id2)
            .ok_or("旅行不存在".to_string())?;
        let photos = db2.get_trip_photos(&trip.id).map_err(|e| e.to_string())?;
        // 获取所有照片的标签（人物等）
        let all_tags = db2.get_all_photo_tags().unwrap_or_default();
        // 把 tag_id 转成 tag name
        let mut photo_tags_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        if let Ok(tags_list) = db2.get_tags() {
            let tag_name_map: std::collections::HashMap<String, String> = tags_list
                .into_iter()
                .map(|t| (t.id, t.name))
                .collect();
            for (photo_id, tag_ids) in all_tags {
                let names: Vec<String> = tag_ids
                    .into_iter()
                    .filter_map(|tid| tag_name_map.get(&tid).cloned())
                    .collect();
                if !names.is_empty() {
                    photo_tags_map.insert(photo_id, names);
                }
            }
        }
        Ok::<_, String>((trip, photos, photo_tags_map))
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit("ai-journal-progress", serde_json::json!({ "type": "status", "text": "正在分析旅行数据..." }));

    let prompt = build_journal_prompt(&trip, &photos, &photo_tags_map);

    let _ = app.emit("ai-journal-progress", serde_json::json!({ "type": "status", "text": "正在生成游记..." }));

    let full_text = call_ai_api(&app, &provider, &api_key, &model, &base_url, &prompt).await?;

    let db3 = state.db.clone();
    let trip_id3 = trip_id.clone();
    let full_text3 = full_text.clone();
    tokio::task::spawn_blocking(move || {
        db3.update_trip_journal(&trip_id3, &full_text3, "ai_generated").map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit("ai-journal-progress", serde_json::json!({ "type": "done", "text": full_text }));

    Ok(full_text)
}

#[tauri::command]
pub async fn polish_ai_journal(
    app: AppHandle,
    trip_id: String,
    original_text: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.clone();

    let provider = db.get_config("ai_provider").unwrap_or(None).unwrap_or_else(|| "openai".to_string());
    let api_key = db.get_config("ai_api_key").unwrap_or(None).unwrap_or_default();
    let model = db.get_config("ai_model").unwrap_or(None).unwrap_or_else(|| "gpt-4o-mini".to_string());
    let base_url = db.get_config("ai_base_url").unwrap_or(None).unwrap_or_default();

    if original_text.trim().is_empty() {
        return Err("请先写一些内容再润色".to_string());
    }

    let db2 = state.db.clone();
    let trip_id2 = trip_id.clone();
    let trip = tokio::task::spawn_blocking(move || {
        let trip = db2.get_all_trips()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|t| t.id == trip_id2)
            .ok_or("旅行不存在".to_string())?;
        Ok::<_, String>(trip)
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit("ai-journal-progress", serde_json::json!({ "type": "status", "text": "正在润色游记..." }));

    let prompt = build_polish_prompt(&trip, &original_text);

    let full_text = call_ai_api(&app, &provider, &api_key, &model, &base_url, &prompt).await?;

    let db3 = state.db.clone();
    let trip_id3 = trip_id.clone();
    let full_text3 = full_text.clone();
    tokio::task::spawn_blocking(move || {
        db3.update_trip_journal(&trip_id3, &full_text3, "ai_polished").map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit("ai-journal-progress", serde_json::json!({ "type": "done", "text": full_text }));

    Ok(full_text)
}

fn build_polish_prompt(trip: &Trip, original_text: &str) -> String {
    let cities: Vec<&str> = trip.city_names
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<&str>>(s).ok())
        .unwrap_or_default();

    let city_str = if cities.is_empty() { "".to_string() } else { format!("，地点在{}", cities.join("、")) };

    format!(
        r#"你是一位资深旅行作家和编辑。请对以下用户手写的游记进行润色和优化。

旅行背景：{}至{}{}

用户原文：
---
{}
---

润色要求：
1. 保持原文的核心内容、情感和个人风格，不要改变原意
2. 优化语言表达，使文字更流畅、更有文采
3. 适当增加细节描写和画面感
4. 调整段落结构，使逻辑更清晰
5. 修正可能的语法错误和不通顺的地方
6. 润色后的字数可以适当增加，但不要超过原文的 1.5 倍
7. 以第一人称写作
8. 不要使用 Markdown 标题语法，用自然段落即可
9. 不要输出除润色后游记正文以外的任何内容，不要加"润色后"等说明文字"#,
        trip.start_date.as_deref().unwrap_or(""),
        trip.end_date.as_deref().unwrap_or(""),
        city_str,
        original_text,
    )
}

fn build_journal_prompt(
    trip: &Trip,
    photos: &[Photo],
    photo_tags_map: &std::collections::HashMap<String, Vec<String>>,
) -> String {
    let days = if let (Some(start), Some(end)) = (&trip.start_date, &trip.end_date) {
        if let (Ok(s), Ok(e)) = (
            chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d"),
            chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d"),
        ) {
            (e - s).num_days() + 1
        } else { 1 }
    } else { 1 };

    let cities: Vec<&str> = trip.city_names
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<&str>>(s).ok())
        .unwrap_or_default();

    let cameras: Vec<String> = photos.iter()
        .filter_map(|p| p.camera_model.as_deref())
        .map(|s| s.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let camera_str = if cameras.is_empty() { "未知".to_string() } else { cameras.join("、") };
    let city_str = if cities.is_empty() { "未知地点".to_string() } else { cities.join("、") };

    // 统计所有出现过的人物
    let mut all_people: std::collections::HashSet<String> = std::collections::HashSet::new();
    for names in photo_tags_map.values() {
        for name in names {
            all_people.insert(name.clone());
        }
    }
    let people_str = if all_people.is_empty() {
        "未识别人物".to_string()
    } else {
        all_people.iter().cloned().collect::<Vec<_>>().join("、")
    };

    // 提供每张照片的详细信息，按时间排序，最多选 30 张代表性照片
    let mut sorted_photos: Vec<(usize, &Photo)> = photos.iter().enumerate().collect();
    sorted_photos.sort_by(|a, b| {
        let ta = a.1.taken_time.as_deref().unwrap_or("");
        let tb = b.1.taken_time.as_deref().unwrap_or("");
        ta.cmp(tb)
    });

    let step = (sorted_photos.len() as f32 / 30.0).max(1.0) as usize;
    let selected: Vec<(usize, &Photo)> = sorted_photos
        .into_iter()
        .step_by(step)
        .take(30)
        .collect();

    let mut photo_info = String::new();
    for (display_idx, (orig_idx, photo)) in selected.iter().enumerate() {
        let time = photo.taken_time.as_deref().unwrap_or("未知时间");
        let location = if let Some(addr) = &photo.address {
            addr.clone()
        } else {
            let parts: Vec<&str> = [
                photo.province.as_deref(),
                photo.city.as_deref(),
                photo.district.as_deref(),
            ].into_iter().flatten().collect();
            if parts.is_empty() { "未知地点".to_string() } else { parts.join("") }
        };
        let people = photo_tags_map
            .get(&photo.id)
            .map(|names| names.join("、"))
            .unwrap_or_else(|| "无".to_string());
        let camera = photo.camera_model.as_deref().unwrap_or("未知");

        photo_info.push_str(&format!(
            "[{}] 时间:{}\n     地点:{}\n     人物:{}\n     设备:{}\n     文件名:{}\n\n",
            display_idx + 1,
            time,
            location,
            people,
            camera,
            photo.file_name
        ));
    }

    // 建立显示编号到原始索引的映射（用于提示 AI 正确的 photo 编号）
    let index_map: Vec<usize> = selected.iter().map(|(idx, _)| *idx + 1).collect();
    let index_hint = index_map
        .iter()
        .enumerate()
        .map(|(i, orig)| format!("[{}]→photo:{}", i + 1, orig))
        .collect::<Vec<_>>()
        .join("，");

    format!(
        r#"你是一位旅行作家。请根据以下旅行数据，写一篇约800-1200字的图文游记。

旅行信息：
- 时间：{} 至 {}（{} 天）
- 地点：{}
- 照片数：{} 张
- 同行人物：{}
- 拍摄设备：{}

照片详细列表（按时间排序，编号对应可插入的图片）：
{}

照片编号映射：{}
（说明：上面列表中的 [N] 编号对应插入语法 [photo:M]，请使用映射后的 M 值）

要求：
1. 以第一人称写作
2. 按时间线展开叙述
3. 语言自然生动，有情感和细节
4. 结合照片中的人物、地点、时间信息，让游记有画面感
5. 结尾有感想总结
6. 在合适的段落之间插入图片，格式为 [photo:编号]，例如 [photo:3] 表示插入第3张照片
7. 全篇插入 6-10 张图片，分布均匀，每个图片独占一行
8. 不要使用 Markdown 标题语法，用自然段落即可
9. 不要输出除游记正文以外的任何内容"#,
        trip.start_date.as_deref().unwrap_or("未知"),
        trip.end_date.as_deref().unwrap_or("未知"),
        days,
        city_str,
        photos.len(),
        people_str,
        camera_str,
        photo_info,
        index_hint,
    )
}

async fn call_ai_api(
    app: &AppHandle,
    provider: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    prompt: &str,
) -> Result<String, String> {
    println!("\n========== AI Prompt ==========");
    println!("Provider: {}", provider);
    println!("Model: {}", if model.is_empty() { "(default)" } else { model });
    println!("Prompt length: {} chars", prompt.len());
    println!("-------------------------------");
    println!("{}", prompt);
    println!("===============================\n");

    // 同时发送到前端控制台
    let _ = app.emit("ai-prompt-debug", serde_json::json!({
        "provider": provider,
        "model": if model.is_empty() { "(default)" } else { model },
        "prompt_length": prompt.len(),
        "prompt": prompt,
    }));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let (url, body) = match provider {
        "ollama" => {
            let base = if base_url.is_empty() { "http://localhost:11434" } else { base_url };
            let url = format!("{}/api/generate", base);
            let body = serde_json::json!({
                "model": if model.is_empty() { "qwen2.5:7b" } else { model },
                "prompt": prompt,
                "stream": true,
            });
            (url, body)
        }
        _ => {
            let base = if base_url.is_empty() {
                match provider {
                    "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1",
                    "deepseek" => "https://api.deepseek.com/v1",
                    _ => "https://api.openai.com/v1",
                }
            } else { base_url };
            let url = format!("{}/chat/completions", base);
            let default_model = match provider {
                "deepseek" => "deepseek-chat",
                "qwen" => "qwen-plus",
                _ => "gpt-4o-mini",
            };
            let body = serde_json::json!({
                "model": if model.is_empty() { default_model } else { model },
                "messages": [{"role": "user", "content": prompt}],
                "stream": true,
            });
            (url, body)
        }
    };

    let mut req = client.post(&url).json(&body);
    if provider != "ollama" && !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }

    let mut response = req.send().await.map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API 返回错误 ({}): {}", status, text));
    }

    let mut full_text = String::new();
    let mut buf = String::new();

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if line.is_empty() { continue; }

            if provider == "ollama" {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(content) = json.get("response").and_then(|v| v.as_str()) {
                        full_text.push_str(content);
                        let _ = app.emit("ai-journal-progress", serde_json::json!({
                            "type": "chunk", "text": content
                        }));
                    }
                    if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                        break;
                    }
                }
            } else {
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { break; }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = json
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|v| v.as_str())
                    {
                        full_text.push_str(content);
                        let _ = app.emit("ai-journal-progress", serde_json::json!({
                            "type": "chunk", "text": content
                        }));
                    }
                }
            }
        }
    }

    if full_text.is_empty() {
        return Err("AI 未返回任何内容".to_string());
    }

    Ok(full_text)
}

// ---------- 影集生成 ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateSlideshowRequest {
    pub trip_id: String,
    pub title: String,
    pub narration: String,
    pub photo_paths: Vec<String>,
    pub output_path: String,
    pub photo_duration: f32,
    pub transition_duration: f32,
    pub width: u32,
    pub height: u32,
    pub music_path: Option<String>,
}

#[tauri::command]
pub async fn generate_slideshow(
    request: GenerateSlideshowRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = crate::slideshow::SlideshowConfig {
        trip_id: request.trip_id,
        title: request.title,
        narration: request.narration,
        photo_paths: request.photo_paths,
        output_path: request.output_path.clone(),
        photo_duration: request.photo_duration,
        transition_duration: request.transition_duration,
        width: request.width,
        height: request.height,
        music_path: request.music_path,
    };

    let progress = Arc::new(AtomicUsize::new(0));

    tokio::task::spawn_blocking(move || {
        crate::slideshow::generate_slideshow(&config, app, progress)
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(request.output_path)
}

#[tauri::command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    crate::export::open_file_location(&path)
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeFacesResult {
    pub total_photos: usize,
    pub photos_with_faces: usize,
    pub total_faces: usize,
    pub persons_found: usize,
}

#[tauri::command]
pub async fn analyze_faces(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AnalyzeFacesResult, String> {
    let db = state.db.clone();
    let thumbs_dir = state.thumbs_dir.clone();

    let photos = tokio::task::spawn_blocking(move || db.get_all_photos().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let total = photos.len();
    if total == 0 {
        return Ok(AnalyzeFacesResult {
            total_photos: 0,
            photos_with_faces: 0,
            total_faces: 0,
            persons_found: 0,
        });
    }

    let app_arc = Arc::new(app);
    let processed = Arc::new(AtomicUsize::new(0));
    let db_arc = state.db.clone();

    // 清除上次自动生成的标签（保留手动创建的），并删除旧的人脸缩略图文件
    let old_face_thumbs = db_arc.delete_auto_person_tags().map_err(|e| e.to_string())?;
    for thumb in old_face_thumbs {
        if thumb.starts_with(&thumbs_dir) {
            let _ = std::fs::remove_file(&thumb);
        }
    }

    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().min(3))
        .build()
        .unwrap();

    let all_detections: Vec<face::FaceDetection> = thread_pool.install(|| {
        photos
            .par_iter()
            .filter_map(|photo| {
                let img = match crate::image_utils::open_with_orientation(&photo.file_path).ok() {
                    Some(img) => img,
                    None => {
                        let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 10 == 0 || count == total {
                            let _ = app_arc.emit("face-analyze-progress", (count, total));
                        }
                        return None;
                    }
                };

                let faces = face::detect_faces(&img);
                let mut detections = Vec::new();

                for face_rect in &faces {
                    let features = face::extract_face_features(&img, face_rect);
                    detections.push(face::FaceDetection {
                        rect: face_rect.clone(),
                        features,
                        photo_id: photo.id.clone(),
                    });
                }

                if !detections.is_empty() && !faces.is_empty() {
                    let _ = face::save_face_thumbnail(&img, &faces[0], &thumbs_dir, &photo.id);
                }

                let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                if count % 5 == 0 || count == total {
                    let _ = app_arc.emit("face-analyze-progress", (count, total));
                }

                if detections.is_empty() {
                    None
                } else {
                    Some(detections)
                }
            })
            .flatten()
            .collect()
    });

    let total_faces = all_detections.len();
    let photos_with_faces = all_detections
        .iter()
        .map(|d| &d.photo_id)
        .collect::<std::collections::HashSet<_>>()
        .len();

    eprintln!("=== 人脸分析结果 ===");
    eprintln!("总照片数: {}", total);
    eprintln!("含人脸照片数: {}", photos_with_faces);
    eprintln!("总人脸检测数: {}", total_faces);

    let clusters = face::cluster_faces(all_detections, 0.82);

    eprintln!("聚类数: {}", clusters.len());
    for (i, c) in clusters.iter().enumerate() {
        let unique_photos: std::collections::HashSet<_> = c.iter().map(|d| &d.photo_id).collect();
        eprintln!("  聚类{}: {} 张人脸, {} 张照片", i + 1, c.len(), unique_photos.len());
    }

    let person_colors = [
        "#e89a5a", "#5ba882", "#d47070", "#8b5cf6",
        "#ec4899", "#14b8a6", "#f97316", "#6366f1",
        "#f59e0b", "#10b981",
    ];

    for (i, cluster) in clusters.iter().enumerate() {
        if cluster.is_empty() {
            continue;
        }

        let person_num = i + 1;
        let name = format!("人物 {}", person_num);
        let color = person_colors[i % person_colors.len()];

        let first_detection = &cluster[0];
        let first_photo = photos.iter().find(|p| p.id == first_detection.photo_id);

        let face_thumb = if let Some(photo) = first_photo {
            let img = crate::image_utils::open_with_orientation(&photo.file_path).ok();
            if let Some(ref img) = img {
                face::save_face_thumbnail(img, &first_detection.rect, &thumbs_dir, &first_detection.photo_id)
            } else {
                None
            }
        } else {
            None
        };

        eprintln!("  创建标签: {} (face_thumb={})", name, face_thumb.is_some());

        let tag = db_arc.get_or_create_person_tag(&name, color, face_thumb.as_deref())
            .map_err(|e| e.to_string())?;

        let photo_ids: Vec<String> = cluster.iter()
            .map(|d| d.photo_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        db_arc.add_tag_to_photos(&tag.id, &photo_ids)
            .map_err(|e| e.to_string())?;

        eprintln!("  标签已创建: {}, 关联照片: {}", tag.name, photo_ids.len());
    }

    let persons_found = clusters.len();

    Ok(AnalyzeFacesResult {
        total_photos: total,
        photos_with_faces,
        total_faces,
        persons_found,
    })
}

#[tauri::command]
pub async fn update_tag(
    id: String,
    name: Option<String>,
    description: Option<String>,
    face_thumb: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.update_tag(&id, name.as_deref(), description.as_deref(), face_thumb.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_tag_photos(
    tag_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_tag_photos(&tag_id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
