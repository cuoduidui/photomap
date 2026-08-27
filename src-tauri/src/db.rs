use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;
use crate::geocode::GeocodeResult;

pub struct Database {
    pub conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub taken_time: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
    pub is_location_manual: bool,
    pub camera_model: Option<String>,
    pub exif_data: Option<String>,
    pub thumbnail_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomLocation {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoStats {
    pub total: i64,
    pub located: i64,
    pub unlocated: i64,
    pub cities: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationCount {
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
    pub count: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub tag_type: String,
    pub color: String,
    pub count: Option<i64>,
    pub description: Option<String>,
    pub face_thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: String,
    pub title: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub cover_photo_id: Option<String>,
    pub city_names: Option<String>,
    pub province_names: Option<String>,
    pub photo_count: i64,
    pub journal_text: Option<String>,
    pub journal_type: Option<String>,
    pub is_manual: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoCoord {
    pub id: String,
    pub taken_time: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub province: Option<String>,
    pub city: Option<String>,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
}

/// 生成地点分组显示名：优先「区县·街道」，次选区县，再选街道，最后回退城市。
fn combine_location_display(
    city: &Option<String>,
    district: &Option<String>,
    address: &Option<String>,
) -> Option<String> {
    match (district, address) {
        (Some(d), Some(a)) => Some(format!("{}·{}", d, a)),
        (Some(d), None) => Some(d.clone()),
        (None, Some(a)) => Some(a.clone()),
        (None, None) => city.clone(),
    }
}

impl Database {
    pub fn open(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let parent = Path::new(db_path).parent();
        if let Some(p) = parent {
            std::fs::create_dir_all(p)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS photos (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                file_size INTEGER,
                taken_time TEXT,
                latitude REAL,
                longitude REAL,
                province TEXT,
                city TEXT,
                district TEXT,
                address TEXT,
                is_location_manual INTEGER DEFAULT 0,
                camera_model TEXT,
                exif_data TEXT,
                thumbnail_path TEXT,
                created_at TEXT,
                updated_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_photos_taken_time ON photos(taken_time);
            CREATE INDEX IF NOT EXISTS idx_photos_location ON photos(province, city, district);
            CREATE INDEX IF NOT EXISTS idx_photos_coords ON photos(latitude, longitude);

            CREATE TABLE IF NOT EXISTS custom_locations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                address TEXT,
                created_at TEXT
            );

            CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                tag_type TEXT DEFAULT 'custom',
                color TEXT DEFAULT '#0ea5e9',
                created_at TEXT
            );

            CREATE TABLE IF NOT EXISTS photo_tags (
                photo_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (photo_id, tag_id),
                FOREIGN KEY (photo_id) REFERENCES photos(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_photo_tags_photo ON photo_tags(photo_id);
            CREATE INDEX IF NOT EXISTS idx_photo_tags_tag ON photo_tags(tag_id);

            CREATE TABLE IF NOT EXISTS trips (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                start_date TEXT,
                end_date TEXT,
                cover_photo_id TEXT,
                city_names TEXT,
                province_names TEXT,
                photo_count INTEGER DEFAULT 0,
                journal_text TEXT,
                journal_type TEXT,
                is_manual INTEGER DEFAULT 0,
                created_at TEXT,
                updated_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_trips_dates ON trips(start_date, end_date);"
        )?;

        conn.execute_batch(
            "ALTER TABLE photos ADD COLUMN trip_id TEXT REFERENCES trips(id);"
        ).ok();

        conn.execute_batch(
            "ALTER TABLE trips ADD COLUMN is_manual INTEGER DEFAULT 0;"
        ).ok();

        conn.execute_batch(
            "ALTER TABLE tags ADD COLUMN description TEXT;"
        ).ok();

        conn.execute_batch(
            "ALTER TABLE tags ADD COLUMN face_thumb TEXT;"
        ).ok();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS geocode_cache (
                cache_key TEXT PRIMARY KEY,
                province TEXT,
                city TEXT,
                district TEXT,
                township TEXT,
                street TEXT,
                address TEXT,
                formatted_address TEXT
            );"
        ).ok();

        Ok(())
    }

    pub fn insert_photo(&self, photo: &Photo) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO photos (id, file_path, file_name, file_size, taken_time,
             latitude, longitude, province, city, district, address, is_location_manual,
             camera_model, exif_data, thumbnail_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                photo.id, photo.file_path, photo.file_name, photo.file_size,
                photo.taken_time, photo.latitude, photo.longitude,
                photo.province, photo.city, photo.district, photo.address,
                photo.is_location_manual as i32, photo.camera_model, photo.exif_data,
                photo.thumbnail_path, photo.created_at, photo.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn insert_photos_batch(&self, photos: &[Photo]) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut success = 0usize;
        let mut skipped = 0usize;
        let chunk_size = 200;
        for chunk in photos.chunks(chunk_size) {
            let tx = conn.unchecked_transaction()?;
            for photo in chunk {
                match tx.execute(
                    "INSERT OR IGNORE INTO photos (id, file_path, file_name, file_size, taken_time,
                     latitude, longitude, province, city, district, address, is_location_manual,
                     camera_model, exif_data, thumbnail_path, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                    params![
                        photo.id, photo.file_path, photo.file_name, photo.file_size,
                        photo.taken_time, photo.latitude, photo.longitude,
                        photo.province, photo.city, photo.district, photo.address,
                        photo.is_location_manual as i32, photo.camera_model, photo.exif_data,
                        photo.thumbnail_path, photo.created_at, photo.updated_at,
                    ],
                ) {
                    Ok(rows) => {
                        if rows > 0 { success += 1; } else { skipped += 1; }
                    }
                    Err(e) => {
                        let msg = e.to_string().to_lowercase();
                        if msg.contains("unique") || msg.contains("duplicate") {
                            skipped += 1;
                        }
                    }
                }
            }
            tx.commit()?;
        }
        Ok((success, skipped))
    }

    pub fn update_photo_location(
        &self,
        id: &str,
        lat: f64,
        lng: f64,
        province: Option<&str>,
        city: Option<&str>,
        district: Option<&str>,
        address: Option<&str>,
        is_manual: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE photos SET latitude=?1, longitude=?2, province=?3, city=?4, district=?5,
             address=?6, is_location_manual=?7, updated_at=?8 WHERE id=?9",
            params![lat, lng, province, city, district, address, is_manual as i32, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn get_all_photos(&self) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos ORDER BY taken_time DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    pub fn get_photo_by_id(&self, id: &str) -> Result<Option<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos WHERE id=?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn get_photos_without_address(&self) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos
             WHERE latitude IS NOT NULL AND longitude IS NOT NULL AND (province IS NULL OR city IS NULL)"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    pub fn delete_photo(&self, id: &str) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // 先查询文件路径，便于命令层安全删除磁盘文件
        let paths: Option<(Option<String>, Option<String>)> = {
            let mut stmt = conn.prepare("SELECT file_path, thumbnail_path FROM photos WHERE id=?1")?;
            let mut rows = stmt.query_map(params![id], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?))
            })?;
            rows.next().transpose()?
        };
        conn.execute("DELETE FROM photos WHERE id=?1", params![id])?;
        Ok(paths.unwrap_or((None, None)))
    }

    /// 分页获取照片（PRD 目标约 1 万张，避免一次性加载全表导致内存暴涨）
    pub fn get_photos_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos ORDER BY taken_time DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    pub fn count_photos(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM photos", [], |r| r.get(0))?;
        Ok(count)
    }

    /// 从持久化缓存读取反地理编码结果（重启后依然有效，减少高德 API 调用）
    pub fn get_geocode_cache(&self, lat: f64, lng: f64) -> Result<Option<GeocodeResult>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let key = format!("{:.4},{:.4}", lat, lng);
        let mut stmt = conn.prepare(
            "SELECT province, city, district, township, street, address, formatted_address
             FROM geocode_cache WHERE cache_key=?1"
        )?;
        let mut rows = stmt.query_map(params![key], |row| {
            Ok(GeocodeResult {
                province: row.get(0)?,
                city: row.get(1)?,
                district: row.get(2)?,
                township: row.get(3)?,
                street: row.get(4)?,
                address: row.get(5)?,
                formatted_address: row.get(6)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// 写入持久化反地理编码缓存
    pub fn set_geocode_cache(&self, lat: f64, lng: f64, result: &GeocodeResult) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let key = format!("{:.4},{:.4}", lat, lng);
        conn.execute(
            "INSERT OR REPLACE INTO geocode_cache (cache_key, province, city, district, township, street, address, formatted_address)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key, result.province, result.city, result.district, result.township, result.street, result.address, result.formatted_address
            ],
        )?;
        Ok(())
    }

    pub fn get_stats(&self) -> Result<PhotoStats, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM photos", [], |r| r.get(0))?;
        let located: i64 = conn.query_row(
            "SELECT COUNT(*) FROM photos WHERE latitude IS NOT NULL", [], |r| r.get(0),
        )?;
        let cities: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT city) FROM photos WHERE city IS NOT NULL", [], |r| r.get(0),
        )?;
        Ok(PhotoStats {
            total,
            located,
            unlocated: total - located,
            cities,
        })
    }

    pub fn get_location_counts(&self) -> Result<Vec<LocationCount>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut results = Vec::new();

        // 先查有省市区信息的，按 省/市/区/街道 逐级分组（第二级展示为街道级地点名）
        let mut stmt = conn.prepare(
            "SELECT province, city, district, address,
                    AVG(latitude), AVG(longitude), COUNT(*) as cnt
             FROM photos
             WHERE province IS NOT NULL AND city IS NOT NULL
             GROUP BY province, city, district, address
             ORDER BY cnt DESC LIMIT 60"
        )?;
        let rows = stmt.query_map([], |row| {
            let province: Option<String> = row.get(0)?;
            let city: Option<String> = row.get(1)?;
            let district: Option<String> = row.get(2)?;
            let address: Option<String> = row.get(3)?;
            let lat: Option<f64> = row.get(4)?;
            let lng: Option<f64> = row.get(5)?;
            let cnt: i64 = row.get(6)?;
            let display = combine_location_display(&city, &district, &address);
            Ok(LocationCount {
                province,
                city: display,
                district,
                address,
                count: cnt,
                latitude: lat,
                longitude: lng,
            })
        })?;
        for row in rows {
            results.push(row?);
        }
        drop(stmt);

        // 再查有GPS但无省市区的，按坐标区域（保留2位小数，约1公里精度）分组
        let mut stmt2 = conn.prepare(
            "SELECT 
               ROUND(latitude, 2) as lat_group,
               ROUND(longitude, 2) as lng_group,
               AVG(latitude),
               AVG(longitude),
               COUNT(*) as cnt
             FROM photos
             WHERE latitude IS NOT NULL AND longitude IS NOT NULL
               AND (province IS NULL OR city IS NULL)
             GROUP BY lat_group, lng_group
             ORDER BY cnt DESC LIMIT 30"
        )?;
        let rows2 = stmt2.query_map([], |row| {
            let lat: f64 = row.get(2)?;
            let lng: f64 = row.get(3)?;
            let city_name = format!("坐标 {:.2}, {:.2}", lat, lng);
            Ok(LocationCount {
                province: Some("坐标地点".to_string()),
                city: Some(city_name),
                district: None,
                address: None,
                latitude: Some(lat),
                longitude: Some(lng),
                count: row.get(4)?,
            })
        })?;
        for row in rows2 {
            results.push(row?);
        }

        // 按数量排序
        results.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(results)
    }

    pub fn get_custom_locations(&self) -> Result<Vec<CustomLocation>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, latitude, longitude, address, created_at FROM custom_locations ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomLocation {
                id: row.get(0)?,
                name: row.get(1)?,
                latitude: row.get(2)?,
                longitude: row.get(3)?,
                address: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn add_custom_location(
        &self,
        name: &str,
        lat: f64,
        lng: f64,
        address: Option<&str>,
    ) -> Result<CustomLocation, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let loc = CustomLocation {
            id: id.clone(),
            name: name.to_string(),
            latitude: lat,
            longitude: lng,
            address: address.map(|s| s.to_string()),
            created_at: now.clone(),
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO custom_locations (id, name, latitude, longitude, address, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, lat, lng, address, now],
        )?;
        Ok(loc)
    }

    pub fn delete_custom_location(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM custom_locations WHERE id=?1", params![id])?;
        Ok(())
    }

    // --- Tags ---
    pub fn get_tags(&self) -> Result<Vec<Tag>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.tag_type, t.color, COUNT(pt.photo_id) as cnt, t.description, t.face_thumb
             FROM tags t LEFT JOIN photo_tags pt ON t.id = pt.tag_id
             GROUP BY t.id ORDER BY t.name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                tag_type: row.get(2)?,
                color: row.get(3)?,
                count: row.get(4)?,
                description: row.get(5)?,
                face_thumb: row.get(6)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn create_tag(&self, name: &str, tag_type: &str, color: &str) -> Result<Tag, Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tags (id, name, tag_type, color, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, tag_type, color, now],
        )?;
        Ok(Tag { id, name: name.to_string(), tag_type: tag_type.to_string(), color: color.to_string(), count: Some(0), description: None, face_thumb: None })
    }

    pub fn create_person_tag(&self, name: &str, color: &str, face_thumb: Option<&str>) -> Result<Tag, Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tags (id, name, tag_type, color, face_thumb, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, "person", color, face_thumb, now],
        )?;
        Ok(Tag { id, name: name.to_string(), tag_type: "person".to_string(), color: color.to_string(), count: Some(0), description: None, face_thumb: face_thumb.map(|s| s.to_string()) })
    }

    pub fn update_tag(&self, id: &str, name: Option<&str>, description: Option<&str>, face_thumb: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        if let Some(n) = name {
            conn.execute("UPDATE tags SET name=?1 WHERE id=?2", params![n, id])?;
        }
        if let Some(d) = description {
            conn.execute("UPDATE tags SET description=?1 WHERE id=?2", params![d, id])?;
        }
        if let Some(ft) = face_thumb {
            conn.execute("UPDATE tags SET face_thumb=?1 WHERE id=?2", params![ft, id])?;
        }
        Ok(())
    }

    pub fn delete_auto_person_tags(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        // 只删除自动生成的人物标签（有face_thumb的），保留手动创建的
        conn.execute("DELETE FROM tags WHERE tag_type='person' AND face_thumb IS NOT NULL", [])?;
        Ok(())
    }

    pub fn get_or_create_person_tag(&self, name: &str, color: &str, face_thumb: Option<&str>) -> Result<Tag, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();

        // 先查找是否已存在同名标签
        let mut stmt = conn.prepare("SELECT id, name, tag_type, color, description, face_thumb FROM tags WHERE name=?1 AND tag_type='person'")?;
        let existing = stmt.query_row(params![name], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                tag_type: row.get(2)?,
                color: row.get(3)?,
                count: Some(0),
                description: row.get(4)?,
                face_thumb: row.get(5)?,
            })
        }).ok();

        if let Some(tag) = existing {
            // 更新face_thumb
            if let Some(thumb) = face_thumb {
                conn.execute("UPDATE tags SET face_thumb=?1 WHERE id=?2", params![thumb, tag.id])?;
            }
            return Ok(Tag { face_thumb: face_thumb.map(|s| s.to_string()).or(tag.face_thumb), ..tag });
        }

        // 创建新标签
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tags (id, name, tag_type, color, face_thumb, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, "person", color, face_thumb, now],
        )?;

        Ok(Tag {
            id,
            name: name.to_string(),
            tag_type: "person".to_string(),
            color: color.to_string(),
            count: Some(0),
            description: None,
            face_thumb: face_thumb.map(|s| s.to_string()),
        })
    }

    pub fn get_tag_photos(&self, tag_id: &str) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT p.* FROM photos p
             INNER JOIN photo_tags pt ON p.id = pt.photo_id
             WHERE pt.tag_id = ?1
             ORDER BY p.taken_time ASC"
        )?;
        let rows = stmt.query_map(params![tag_id], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn delete_tag(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tags WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn add_photo_tags(&self, photo_id: &str, tag_ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        for tag_id in tag_ids {
            tx.execute(
                "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
                params![photo_id, tag_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn add_tag_to_photos(&self, tag_id: &str, photo_ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        for photo_id in photo_ids {
            tx.execute(
                "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
                params![photo_id, tag_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn remove_photo_tag(&self, photo_id: &str, tag_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM photo_tags WHERE photo_id=?1 AND tag_id=?2",
            params![photo_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_photo_tags(&self, photo_id: &str) -> Result<Vec<Tag>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.tag_type, t.color, t.description, t.face_thumb FROM tags t
             INNER JOIN photo_tags pt ON t.id = pt.tag_id
             WHERE pt.photo_id = ?1 ORDER BY t.name ASC"
        )?;
        let rows = stmt.query_map(params![photo_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                tag_type: row.get(2)?,
                color: row.get(3)?,
                count: None,
                description: row.get(4)?,
                face_thumb: row.get(5)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_all_photo_tags(&self) -> Result<std::collections::HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT photo_id, tag_id FROM photo_tags"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for row in rows {
            let (photo_id, tag_id) = row?;
            map.entry(photo_id).or_insert_with(Vec::new).push(tag_id);
        }
        Ok(map)
    }

    pub fn get_config(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_config WHERE key=?1")?;
        let mut rows = stmt.query_map(params![key], |r| r.get::<_, String>(0))?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_config (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_photo_coords(&self) -> Result<Vec<PhotoCoord>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, taken_time, latitude, longitude, province, city, file_path, thumbnail_path
             FROM photos WHERE latitude IS NOT NULL AND longitude IS NOT NULL
             ORDER BY taken_time ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PhotoCoord {
                id: row.get(0)?,
                taken_time: row.get(1)?,
                latitude: row.get(2)?,
                longitude: row.get(3)?,
                province: row.get(4)?,
                city: row.get(5)?,
                file_path: row.get(6)?,
                thumbnail_path: row.get(7)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_cities_count(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT city) FROM photos WHERE city IS NOT NULL",
            [], |r| r.get(0),
        )?;
        Ok(count)
    }

    pub fn insert_trip(&self, trip: &Trip) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trips (id, title, start_date, end_date, cover_photo_id, city_names,
             province_names, photo_count, journal_text, journal_type, is_manual, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                trip.id, trip.title, trip.start_date, trip.end_date, trip.cover_photo_id,
                trip.city_names, trip.province_names, trip.photo_count,
                trip.journal_text, trip.journal_type, trip.is_manual,
                trip.created_at, trip.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn create_trip(&self, title: &str, start_date: Option<&str>, end_date: Option<&str>) -> Result<Trip, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let trip = Trip {
            id: id.clone(),
            title: title.to_string(),
            start_date: start_date.map(|s| s.to_string()),
            end_date: end_date.map(|s| s.to_string()),
            cover_photo_id: None,
            city_names: None,
            province_names: None,
            photo_count: 0,
            journal_text: None,
            journal_type: None,
            is_manual: true,
            created_at: now.clone(),
            updated_at: now,
        };
        self.insert_trip(&trip)?;
        Ok(trip)
    }

    pub fn get_all_trips(&self) -> Result<Vec<Trip>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.title, t.start_date, t.end_date, t.cover_photo_id, t.city_names,
             t.province_names,
             (SELECT COUNT(*) FROM photos p WHERE p.trip_id = t.id) AS actual_photo_count,
             t.journal_text, t.journal_type, t.is_manual, t.created_at, t.updated_at
             FROM trips t ORDER BY t.start_date DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Trip {
                id: row.get(0)?,
                title: row.get(1)?,
                start_date: row.get(2)?,
                end_date: row.get(3)?,
                cover_photo_id: row.get(4)?,
                city_names: row.get(5)?,
                province_names: row.get(6)?,
                photo_count: row.get(7)?,
                journal_text: row.get(8)?,
                journal_type: row.get(9)?,
                is_manual: row.get::<_, i64>(10)? != 0,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn update_photo_trip(&self, photo_id: &str, trip_id: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE photos SET trip_id=?1 WHERE id=?2",
            params![trip_id, photo_id],
        )?;
        Ok(())
    }

    pub fn update_trip_journal(&self, trip_id: &str, text: &str, journal_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE trips SET journal_text=?1, journal_type=?2, updated_at=?3 WHERE id=?4",
            params![text, journal_type, Utc::now().to_rfc3339(), trip_id],
        )?;
        Ok(())
    }

    pub fn delete_trip(&self, trip_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute("UPDATE photos SET trip_id=NULL WHERE trip_id=?1", params![trip_id])?;
        tx.execute("DELETE FROM trips WHERE id=?1", params![trip_id])?;
        tx.commit()?;
        Ok(())
    }

    // 清除所有非手动创建的游记（自动生成的），并释放其照片
    pub fn clear_non_manual_trips(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        // 释放非手动游记的照片
        tx.execute(
            "UPDATE photos SET trip_id=NULL WHERE trip_id IN (SELECT id FROM trips WHERE is_manual = 0)",
            [],
        )?;
        // 删除非手动游记
        tx.execute("DELETE FROM trips WHERE is_manual = 0", [])?;
        tx.commit()?;
        Ok(())
    }

    // 检查照片是否属于手动创建的游记
    pub fn is_photo_in_manual_trip(&self, photo_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM photos p 
             JOIN trips t ON p.trip_id = t.id 
             WHERE p.id = ?1 AND t.is_manual = 1",
            params![photo_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn get_trip_photos(&self, trip_id: &str) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos WHERE trip_id=?1 ORDER BY taken_time ASC"
        )?;
        let rows = stmt.query_map(params![trip_id], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    pub fn add_photos_to_trip(&self, trip_id: &str, photo_ids: &[String]) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut count = 0i64;
        for photo_id in photo_ids {
            let updated = conn.execute(
                "UPDATE photos SET trip_id = ?1, updated_at = datetime('now') 
                 WHERE id = ?2 AND (trip_id IS NULL OR trip_id != ?1)",
                params![trip_id, photo_id],
            )?;
            if updated > 0 {
                count += 1;
            }
        }
        // 更新游记统计
        conn.execute(
            "UPDATE trips SET photo_count = (SELECT COUNT(*) FROM photos WHERE trip_id = ?1),
             updated_at = datetime('now') WHERE id = ?1",
            params![trip_id],
        )?;
        Ok(count)
    }

    pub fn remove_photos_from_trip(&self, trip_id: &str, photo_ids: &[String]) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut count = 0i64;
        for photo_id in photo_ids {
            let updated = conn.execute(
                "UPDATE photos SET trip_id = NULL, updated_at = datetime('now') 
                 WHERE id = ?1 AND trip_id = ?2",
                params![photo_id, trip_id],
            )?;
            if updated > 0 {
                count += 1;
            }
        }
        // 更新游记统计
        conn.execute(
            "UPDATE trips SET photo_count = (SELECT COUNT(*) FROM photos WHERE trip_id = ?1),
             updated_at = datetime('now') WHERE id = ?1",
            params![trip_id],
        )?;
        Ok(count)
    }

    pub fn get_photos_not_in_trip(&self, trip_id: &str, limit: i64, offset: i64) -> Result<Vec<Photo>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, file_size, taken_time, latitude, longitude,
             province, city, district, address, is_location_manual, camera_model, exif_data,
             thumbnail_path, created_at, updated_at FROM photos 
             WHERE trip_id IS NULL OR trip_id != ?1
             ORDER BY taken_time DESC LIMIT ?2 OFFSET ?3"
        )?;
        let rows = stmt.query_map(params![trip_id, limit, offset], |row| {
            Ok(Photo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                taken_time: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                province: row.get(7)?,
                city: row.get(8)?,
                district: row.get(9)?,
                address: row.get(10)?,
                is_location_manual: row.get::<_, i32>(11)? != 0,
                camera_model: row.get(12)?,
                exif_data: row.get(13)?,
                thumbnail_path: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    pub fn count_photos_not_in_trip(&self, trip_id: &str) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM photos WHERE trip_id IS NULL OR trip_id != ?1",
            params![trip_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

pub fn new_photo(file_path: &str, file_name: &str, file_size: Option<i64>) -> Photo {
    let now = Utc::now().to_rfc3339();
    Photo {
        id: Uuid::new_v4().to_string(),
        file_path: file_path.to_string(),
        file_name: file_name.to_string(),
        file_size,
        taken_time: None,
        latitude: None,
        longitude: None,
        province: None,
        city: None,
        district: None,
        address: None,
        is_location_manual: false,
        camera_model: None,
        exif_data: None,
        thumbnail_path: None,
        created_at: now.clone(),
        updated_at: now,
    }
}
