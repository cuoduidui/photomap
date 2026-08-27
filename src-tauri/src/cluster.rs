use serde::{Deserialize, Serialize};
use crate::db::PhotoCoord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPoint {
    pub latitude: f64,
    pub longitude: f64,
    pub count: usize,
    pub photo_ids: Vec<String>,
    pub is_cluster: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

pub fn calculate_bounds(coords: &[PhotoCoord]) -> Option<BoundingBox> {
    if coords.is_empty() {
        return None;
    }
    let mut min_lat = coords[0].latitude;
    let mut max_lat = coords[0].latitude;
    let mut min_lng = coords[0].longitude;
    let mut max_lng = coords[0].longitude;

    for c in coords {
        if c.latitude < min_lat { min_lat = c.latitude; }
        if c.latitude > max_lat { max_lat = c.latitude; }
        if c.longitude < min_lng { min_lng = c.longitude; }
        if c.longitude > max_lng { max_lng = c.longitude; }
    }

    let lat_pad = (max_lat - min_lat) * 0.1;
    let lng_pad = (max_lng - min_lng) * 0.1;

    Some(BoundingBox {
        min_lat: min_lat - lat_pad,
        max_lat: max_lat + lat_pad,
        min_lng: min_lng - lng_pad,
        max_lng: max_lng + lng_pad,
    })
}

pub fn calculate_zoom_level(bounds: &BoundingBox) -> u8 {
    let lat_span = bounds.max_lat - bounds.min_lat;
    let lng_span = bounds.max_lng - bounds.min_lng;
    let span = lat_span.max(lng_span);

    if span > 170.0 { 1 }
    else if span > 80.0 { 2 }
    else if span > 40.0 { 3 }
    else if span > 20.0 { 5 }
    else if span > 10.0 { 6 }
    else if span > 5.0 { 7 }
    else if span > 2.5 { { 8 } }
    else if span > 1.2 { 9 }
    else if span > 0.6 { 10 }
    else if span > 0.3 { 11 }
    else if span > 0.15 { 12 }
    else if span > 0.07 { 13 }
    else if span > 0.03 { 14 }
    else if span > 0.015 { 15 }
    else { 16 }
}

pub fn cluster_photos(
    coords: &[PhotoCoord],
    zoom: u8,
    grid_size: Option<f64>,
) -> Vec<ClusterPoint> {
    if coords.is_empty() {
        return Vec::new();
    }

    let grid = grid_size.unwrap_or_else(|| grid_size_for_zoom(zoom));

    let mut buckets: std::collections::HashMap<(i64, i64), Vec<&PhotoCoord>> =
        std::collections::HashMap::new();

    for c in coords {
        let cell_lat = (c.latitude / grid).floor() as i64;
        let cell_lng = (c.longitude / grid).floor() as i64;
        buckets
            .entry((cell_lat, cell_lng))
            .or_insert_with(Vec::new)
            .push(c);
    }

    let mut points = Vec::new();
    for ((_, _), group) in buckets {
        if group.len() == 1 {
            let c = group[0];
            points.push(ClusterPoint {
                latitude: c.latitude,
                longitude: c.longitude,
                count: 1,
                photo_ids: vec![c.id.clone()],
                is_cluster: false,
            });
        } else {
            let total_lat: f64 = group.iter().map(|c| c.latitude).sum();
            let total_lng: f64 = group.iter().map(|c| c.longitude).sum();
            let n = group.len() as f64;
            points.push(ClusterPoint {
                latitude: total_lat / n,
                longitude: total_lng / n,
                count: group.len(),
                photo_ids: group.iter().map(|c| c.id.clone()).collect(),
                is_cluster: true,
            });
        }
    }

    points
}

fn grid_size_for_zoom(zoom: u8) -> f64 {
    match zoom {
        0..=3 => 10.0,
        4..=5 => 5.0,
        6..=7 => 2.0,
        8..=9 => 1.0,
        10..=11 => 0.5,
        12..=13 => 0.1,
        14..=15 => 0.02,
        16..=17 => 0.005,
        18..=20 => 0.001,
        _ => 0.0005,
    }
}
