use crate::db::{Database, PhotoCoord, Trip};
use chrono::{NaiveDateTime, Datelike};
use uuid::Uuid;
use std::collections::HashSet;

const MIN_PHOTOS_PER_TRIP: usize = 5;
const MAX_TIME_GAP_HOURS: i64 = 48;

pub fn cluster_photos_into_trips(coords: &[PhotoCoord]) -> Vec<TripGroup> {
    let mut groups: Vec<TripGroup> = Vec::new();
    let mut current: Option<TripGroup> = None;

    for coord in coords {
        if coord.taken_time.is_none() {
            continue;
        }
        let time = match parse_time(&coord.taken_time.as_ref().unwrap()) {
            Some(t) => t,
            None => continue,
        };

        let city = coord.city.as_deref().unwrap_or("");
        let province = coord.province.as_deref().unwrap_or("");

        let should_start_new = match &current {
            None => true,
            Some(curr) => {
                let gap = time.signed_duration_since(curr.last_time).num_hours();
                let same_area = same_location_area(&curr.cities, city)
                    && same_location_area(&curr.provinces, province);
                !same_area || gap.abs() > MAX_TIME_GAP_HOURS
            }
        };

        if should_start_new {
            if let Some(g) = current.take() {
                if g.coords.len() >= MIN_PHOTOS_PER_TRIP {
                    groups.push(g);
                }
            }
            let mut cities = HashSet::new();
            let mut provinces = HashSet::new();
            if !city.is_empty() { cities.insert(city.to_string()); }
            if !province.is_empty() { provinces.insert(province.to_string()); }
            current = Some(TripGroup {
                coords: vec![coord.clone()],
                cities,
                provinces,
                first_time: time,
                last_time: time,
            });
        } else {
            let g = current.as_mut().unwrap();
            g.coords.push(coord.clone());
            if !city.is_empty() { g.cities.insert(city.to_string()); }
            if !province.is_empty() { g.provinces.insert(province.to_string()); }
            g.last_time = time;
        }
    }

    if let Some(g) = current {
        if g.coords.len() >= MIN_PHOTOS_PER_TRIP {
            groups.push(g);
        }
    }

    groups
}

fn same_location_area(set: &HashSet<String>, value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if set.is_empty() {
        return false;
    }
    set.contains(value)
}

fn parse_time(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S"))
        .ok()
}

pub struct TripGroup {
    pub coords: Vec<PhotoCoord>,
    pub cities: HashSet<String>,
    pub provinces: HashSet<String>,
    pub first_time: NaiveDateTime,
    pub last_time: NaiveDateTime,
}

impl TripGroup {
    pub fn generate_title(&self) -> String {
        let cities: Vec<&str> = self.cities.iter().map(|s| s.as_str()).collect();
        let provinces: Vec<&str> = self.provinces.iter().map(|s| s.as_str()).collect();

        if cities.len() == 1 {
            let city = cities[0];
            format!(
                "{}月{}日-{}月{}日 {}之旅",
                self.first_time.month(),
                self.first_time.day(),
                self.last_time.month(),
                self.last_time.day(),
                city
            )
        } else if cities.len() > 1 {
            let city_list = cities.join("→");
            format!(
                "{}月{}日-{}月{}日 {}之旅",
                self.first_time.month(),
                self.first_time.day(),
                self.last_time.month(),
                self.last_time.day(),
                city_list
            )
        } else if provinces.len() == 1 {
            format!(
                "{}月{}日-{}月{}日 {}之旅",
                self.first_time.month(),
                self.first_time.day(),
                self.last_time.month(),
                self.last_time.day(),
                provinces[0]
            )
        } else {
            format!(
                "{}月{}日-{}月{}日 旅行",
                self.first_time.month(),
                self.first_time.day(),
                self.last_time.month(),
                self.last_time.day()
            )
        }
    }

    pub fn to_trip(&self) -> Trip {
        let now = chrono::Utc::now().to_rfc3339();
        let cities_json = serde_json::to_string(&self.cities.iter().collect::<Vec<_>>()).unwrap_or_default();
        let provinces_json = serde_json::to_string(&self.provinces.iter().collect::<Vec<_>>()).unwrap_or_default();
        let cover = self.coords.first().map(|c| c.id.clone());

        Trip {
            id: Uuid::new_v4().to_string(),
            title: self.generate_title(),
            start_date: Some(self.first_time.format("%Y-%m-%d").to_string()),
            end_date: Some(self.last_time.format("%Y-%m-%d").to_string()),
            cover_photo_id: cover,
            city_names: Some(cities_json),
            province_names: Some(provinces_json),
            photo_count: self.coords.len() as i64,
            journal_text: None,
            journal_type: None,
            is_manual: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

pub fn run_clustering(db: &Database) -> Result<Vec<Trip>, String> {
    let coords = db.get_photo_coords().map_err(|e| e.to_string())?;
    let groups = cluster_photos_into_trips(&coords);

    // 先清除所有非手动的游记及其照片关联
    db.clear_non_manual_trips().map_err(|e| e.to_string())?;

    let mut trips = Vec::new();
    for group in &groups {
        let trip = group.to_trip();
        db.insert_trip(&trip).map_err(|e| e.to_string())?;
        for coord in &group.coords {
            // 只更新还没有分配到手动游记的照片
            if !db.is_photo_in_manual_trip(&coord.id).map_err(|e| e.to_string())? {
                db.update_photo_trip(&coord.id, Some(&trip.id)).map_err(|e| e.to_string())?;
            }
        }
        trips.push(trip);
    }

    // 把手动游记也加进去返回
    let all_trips = db.get_all_trips().map_err(|e| e.to_string())?;
    Ok(all_trips)
}
