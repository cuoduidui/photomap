use exif::{In, Tag, Value};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifInfo {
    pub taken_time: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub camera_model: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<f64>,
    pub raw_json: Option<String>,
}

pub fn is_supported_image(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp")
}

fn rational_to_f64(r: &exif::Rational) -> f64 {
    r.num as f64 / r.denom as f64
}

pub fn parse_exif(path: &str) -> Option<ExifInfo> {
    let file = File::open(path).ok()?;
    let mut buf_reader = BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    let exif_data = exif_reader.read_from_container(&mut buf_reader).ok()?;

    let taken_time = exif_data
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif_data.get_field(Tag::DateTime, In::PRIMARY))
        .map(|f| {
            let s = f.display_value().to_string();
            normalize_exif_datetime(&s)
        });

    let latitude = extract_gps_coord(&exif_data, Tag::GPSLatitude, Tag::GPSLatitudeRef, 'S');
    let longitude = extract_gps_coord(&exif_data, Tag::GPSLongitude, Tag::GPSLongitudeRef, 'W');

    let camera_model = get_string_field(&exif_data, Tag::Model);
    let iso = get_i32_field(&exif_data, Tag::PhotographicSensitivity);
    let aperture = get_rational_field(&exif_data, Tag::FNumber);
    let shutter_speed = get_shutter_speed(&exif_data);
    let focal_length = get_rational_field(&exif_data, Tag::FocalLength);

    let raw = serde_json::json!({
        "camera_model": camera_model,
        "iso": iso,
        "aperture": aperture,
        "shutter_speed": shutter_speed,
        "focal_length": focal_length,
    });

    Some(ExifInfo {
        taken_time,
        latitude,
        longitude,
        camera_model,
        iso,
        aperture,
        shutter_speed,
        focal_length,
        raw_json: Some(raw.to_string()),
    })
}

fn extract_gps_coord(
    exif_data: &exif::Exif,
    coord_tag: Tag,
    ref_tag: Tag,
    negative_char: char,
) -> Option<f64> {
    let coord_field = exif_data.get_field(coord_tag, In::PRIMARY)?;
    
    let dms: Vec<f64> = match &coord_field.value {
        Value::Rational(arr) if arr.len() == 3 => {
            arr.iter().map(rational_to_f64).collect()
        }
        Value::SRational(arr) if arr.len() == 3 => {
            arr.iter().map(|r| r.num as f64 / r.denom as f64).collect()
        }
        _ => return None,
    };
    
    let deg = dms[0];
    let min = dms[1];
    let sec = dms[2];
    let mut decimal = deg + min / 60.0 + sec / 3600.0;

    let ref_val = exif_data
        .get_field(ref_tag, In::PRIMARY)
        .and_then(|f| f.display_value().to_string().chars().next());
    
    if ref_val == Some(negative_char) {
        decimal = -decimal;
    }
    Some(decimal)
}

fn get_string_field(exif_data: &exif::Exif, tag: Tag) -> Option<String> {
    exif_data
        .get_field(tag, In::PRIMARY)
        .map(|f| f.display_value().to_string())
}

fn get_i32_field(exif_data: &exif::Exif, tag: Tag) -> Option<i32> {
    exif_data.get_field(tag, In::PRIMARY).and_then(|f| {
        match &f.value {
            Value::SShort(v) => v.first().copied().map(|x| x as i32),
            Value::SLong(v) => v.first().copied(),
            Value::Short(v) => v.first().map(|x| *x as i32),
            Value::Long(v) => v.first().map(|x| *x as i32),
            _ => None,
        }
    })
}

fn get_rational_field(exif_data: &exif::Exif, tag: Tag) -> Option<f64> {
    exif_data.get_field(tag, In::PRIMARY).and_then(|f| {
        match &f.value {
            Value::Rational(v) => v.first().map(|r| rational_to_f64(r)),
            Value::SRational(v) => v.first().map(|r| r.num as f64 / r.denom as f64),
            _ => None,
        }
    })
}

fn get_shutter_speed(exif_data: &exif::Exif) -> Option<String> {
    exif_data
        .get_field(Tag::ExposureTime, In::PRIMARY)
        .map(|f| f.display_value().to_string())
}

fn normalize_exif_datetime(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 19 && s.as_bytes()[4] == b':' && s.as_bytes()[7] == b':' {
        format!(
            "{}-{}-{}T{}",
            &s[0..4],
            &s[5..7],
            &s[8..10],
            if s.len() >= 19 { &s[11..19] } else { "00:00:00" }
        )
    } else if s.len() >= 10 && s.as_bytes()[4] == b':' {
        format!("{}-{}-{}T00:00:00", &s[0..4], &s[5..7], &s[8..10])
    } else {
        s.to_string()
    }
}
