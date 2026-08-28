// WGS-84 (GPS/EXIF) → GCJ-02 (高德地图) 坐标转换
// 与后端 src-tauri/src/coord.rs 的实现保持一致

const PI = Math.PI;
const A = 6378245.0;
const EE = 0.00669342162296594323;

function outOfChina(lat, lng) {
  return !(lng >= 72.004 && lng <= 137.8347 && lat >= 0.8293 && lat <= 55.8271);
}

function transformLat(x, y) {
  let ret = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * Math.sqrt(Math.abs(x));
  ret += (20.0 * Math.sin(6.0 * x * PI) + 20.0 * Math.sin(2.0 * x * PI)) * 2.0 / 3.0;
  ret += (20.0 * Math.sin(y * PI) + 40.0 * Math.sin(y / 3.0 * PI)) * 2.0 / 3.0;
  ret += (160.0 * Math.sin(y / 12.0 * PI) + 320.0 * Math.sin(y * PI / 30.0)) * 2.0 / 3.0;
  return ret;
}

function transformLng(x, y) {
  let ret = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * Math.sqrt(Math.abs(x));
  ret += (20.0 * Math.sin(6.0 * x * PI) + 20.0 * Math.sin(2.0 * x * PI)) * 2.0 / 3.0;
  ret += (20.0 * Math.sin(x * PI) + 40.0 * Math.sin(x / 3.0 * PI)) * 2.0 / 3.0;
  ret += (150.0 * Math.sin(x / 12.0 * PI) + 300.0 * Math.sin(x / 30.0 * PI)) * 2.0 / 3.0;
  return ret;
}

export function wgs84ToGcj02(lat, lng) {
  if (outOfChina(lat, lng)) return { lat, lng };
  let dLat = transformLat(lng - 105.0, lat - 35.0);
  let dLng = transformLng(lng - 105.0, lat - 35.0);
  const radLat = (lat / 180.0) * PI;
  let magic = Math.sin(radLat);
  magic = 1.0 - EE * magic * magic;
  const sqrtMagic = Math.sqrt(magic);
  // 与 Rust 实现一致：magic 重新赋值为 sqrtMagic
  const m = sqrtMagic;
  dLat = (dLat * 180.0) / (((A * (1.0 - EE)) / (m * sqrtMagic)) * PI);
  dLng = (dLng * 180.0) / ((A / m) * Math.cos(radLat) * PI);
  return { lat: lat + dLat, lng: lng + dLng };
}

// 返回高德需要的 [lng, lat] 数组
export function toAmapLngLat(lat, lng) {
  const g = wgs84ToGcj02(lat, lng);
  return [g.lng, g.lat];
}
