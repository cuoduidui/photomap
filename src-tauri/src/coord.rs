//! WGS-84 (GPS/EXIF) 与 GCJ-02 (高德/火星坐标系) 之间的转换。
//! EXIF 记录的是 WGS-84 坐标，而高德地图使用 GCJ-02，直接叠加会偏移数百米。

const PI: f64 = 3.1415926535897932384626;
const A: f64 = 6378245.0;
const EE: f64 = 0.00669342162296594323;

fn out_of_china(lat: f64, lng: f64) -> bool {
    !(72.004 <= lng && lng <= 137.8347 && 0.8293 <= lat && lat <= 55.8271)
}

fn transform_lat(x: f64, y: f64) -> f64 {
    let mut ret = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * (x.abs().sqrt());
    ret += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    ret += (20.0 * (y * PI).sin() + 40.0 * (y / 3.0 * PI).sin()) * 2.0 / 3.0;
    ret += (160.0 * (y / 12.0 * PI).sin() + 320.0 * (y * PI / 30.0).sin()) * 2.0 / 3.0;
    ret
}

fn transform_lng(x: f64, y: f64) -> f64 {
    let mut ret = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * (x.abs().sqrt());
    ret += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    ret += (20.0 * (x * PI).sin() + 40.0 * (x / 3.0 * PI).sin()) * 2.0 / 3.0;
    ret += (150.0 * (x / 12.0 * PI).sin() + 300.0 * (x / 30.0 * PI).sin()) * 2.0 / 3.0;
    ret
}

/// 将 WGS-84 坐标转换为 GCJ-02（高德地图）坐标；中国大陆以外地区原样返回。
pub fn wgs84_to_gcj02(lat: f64, lng: f64) -> (f64, f64) {
    if out_of_china(lat, lng) {
        return (lat, lng);
    }
    let d_lat = transform_lat(lng - 105.0, lat - 35.0);
    let d_lng = transform_lng(lng - 105.0, lat - 35.0);
    let rad_lat = lat / 180.0 * PI;
    let magic = (rad_lat).sin();
    let mut magic = 1.0 - EE * magic * magic;
    let sqrt_magic = magic.sqrt();
    magic = sqrt_magic;
    let d_lat = (d_lat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrt_magic) * PI);
    let d_lng = (d_lng * 180.0) / (A / magic * (rad_lat).cos() * PI);
    (lat + d_lat, lng + d_lng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beijing_conversion() {
        // 天安门 WGS-84 坐标，转换后应产生明显偏移（数百米量级）
        let (lat, lng) = wgs84_to_gcj02(39.9087, 116.3975);
        assert!((lat - 39.9087).abs() > 0.0001);
        assert!((lng - 116.3975).abs() > 0.0001);
    }

    #[test]
    fn outside_china_unchanged() {
        let (lat, lng) = wgs84_to_gcj02(40.7128, -74.0060);
        assert_eq!((lat, lng), (40.7128, -74.0060));
    }
}
