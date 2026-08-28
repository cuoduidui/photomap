use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResult {
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub township: Option<String>,
    pub street: Option<String>,
    pub address: Option<String>,
    pub formatted_address: Option<String>,
}

impl GeocodeResult {
    /// 生成街道级展示名：优先街道(township)，回退道路(street)，再回退区县(district)。
    pub fn street_label(&self) -> Option<String> {
        if let Some(t) = &self.township {
            if !t.is_empty() {
                return Some(t.clone());
            }
        }
        if let Some(s) = &self.street {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        self.district.clone()
    }

    /// 构造空结果（反地理编码失败或无 Key 时返回，表示未解析到地址）
    pub fn empty() -> Self {
        GeocodeResult {
            province: None,
            city: None,
            district: None,
            township: None,
            street: None,
            address: None,
            formatted_address: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AmapRegeocodeResponse {
    status: String,
    regeocode: Option<AmapRegeocode>,
}

#[derive(Debug, Deserialize)]
struct AmapRegeocode {
    formatted_address: Option<String>,
    // 高德返回的是驼峰字段 addressComponent，必须显式映射，否则省市区永远解析不出来
    #[serde(rename = "addressComponent")]
    address_component: Option<AmapAddressComponent>,
}

#[derive(Debug, Deserialize)]
struct AmapAddressComponent {
    province: Option<String>,
    city: Option<serde_json::Value>,
    district: Option<String>,
    township: Option<serde_json::Value>,
    street: Option<serde_json::Value>,
}

/// 将可能为 null/数组/对象的 JSON Value 转成字符串
fn value_to_string(v: &Option<serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        _ => None,
    }
}

/// 解析高德 regeo 响应为 GeocodeResult
fn parse_regeocode(regeo: &AmapRegeocode) -> GeocodeResult {
    if let Some(ac) = &regeo.address_component {
        let province = ac.province.clone();
        let district = ac.district.clone();
        // 直辖市（北京/上海/天津/重庆）的高德 city 字段为空对象，回退到区县，再回退到省份，
        // 否则这些照片永远进不了省市区分组，只能以坐标显示
        let city = value_to_string(&ac.city)
            .or_else(|| district.clone())
            .or_else(|| province.clone());
        let township = value_to_string(&ac.township);
        let street = value_to_string(&ac.street);
        GeocodeResult {
            province,
            city,
            district,
            township,
            street,
            address: regeo.formatted_address.clone(),
            formatted_address: regeo.formatted_address.clone(),
        }
    } else {
        GeocodeResult {
            province: None,
            city: None,
            district: None,
            township: None,
            street: None,
            address: regeo.formatted_address.clone(),
            formatted_address: regeo.formatted_address.clone(),
        }
    }
}

pub struct Geocoder {
    client: Client,
    api_key: Arc<Mutex<String>>,
    cache: Arc<Mutex<HashMap<String, GeocodeResult>>>,
    db: Option<Arc<db::Database>>,
}

impl Geocoder {
    pub fn new(api_key: &str, database: Option<Arc<db::Database>>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        Geocoder {
            client,
            api_key: Arc::new(Mutex::new(api_key.to_string())),
            cache: Arc::new(Mutex::new(HashMap::new())),
            db: database,
        }
    }

    /// 热更新高德 Key（无需重启应用）
    pub fn set_api_key(&self, key: &str) {
        let mut guard = self.api_key.lock().unwrap();
        *guard = key.to_string();
    }

    pub fn has_api_key(&self) -> bool {
        !self.api_key.lock().unwrap().is_empty()
    }

    fn cache_key(lat: f64, lng: f64) -> String {
        // v2：修复 addressComponent 解析后让旧缓存失效，强制重新拉取
        format!("v2:{:.4},{:.4}", lat, lng)
    }

    pub async fn reverse_geocode(&self, lat: f64, lng: f64) -> Result<GeocodeResult, Box<dyn std::error::Error>> {
        let key = Self::cache_key(lat, lng);
        {
            let cache = self.cache.lock().unwrap();
            if let Some(result) = cache.get(&key) {
                return Ok(result.clone());
            }
        }

        // 持久化缓存：重启后依然有效，避免重复调用高德 API
        if let Some(db) = &self.db {
            if let Ok(Some(result)) = db.get_geocode_cache(lat, lng) {
                let mut cache = self.cache.lock().unwrap();
                cache.insert(key.clone(), result.clone());
                return Ok(result);
            }
        }

        let api_key = self.api_key.lock().unwrap().clone();
        if api_key.is_empty() {
            return Ok(GeocodeResult::empty());
        }

        let url = "https://restapi.amap.com/v3/geocode/regeo";
        let resp = self
            .client
            .get(url)
            .query(&[
                ("key", &api_key),
                ("location", &format!("{},{}", lng, lat)),
                ("extensions", &"base".to_string()),
                ("output", &"json".to_string()),
            ])
            .send()
            .await?;

        let body: AmapRegeocodeResponse = resp.json().await?;

        if body.status != "1" {
            return Ok(GeocodeResult::empty());
        }

        let result = if let Some(regeo) = body.regeocode {
            parse_regeocode(&regeo)
        } else {
            GeocodeResult::empty()
        };

        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.clone(), result.clone());
        drop(cache);
        if let Some(db) = &self.db {
            let _ = db.set_geocode_cache(lat, lng, &result);
        }

        Ok(result)
    }

    pub async fn batch_reverse_geocode(
        &self,
        coords: Vec<(f64, f64)>,
        progress: impl Fn(usize, usize),
        cancel_flag: Option<&std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Vec<(f64, f64, Option<GeocodeResult>)> {
        let semaphore = Arc::new(Semaphore::new(5));
        let mut results = Vec::with_capacity(coords.len());
        let total = coords.len();

        let tasks: Vec<_> = coords
            .into_iter()
            .map(|(lat, lng)| {
                let sem = semaphore.clone();
                let client = &self.client;
                let api_key = self.api_key.clone();
                let cache = self.cache.clone();
                let db = self.db.clone();
                async move {
                    let _permit = sem.acquire().await;
                    let key = Self::cache_key(lat, lng);
                    // 内存缓存
                    {
                        let cache_guard = cache.lock().unwrap();
                        if let Some(result) = cache_guard.get(&key) {
                            return (lat, lng, Some(result.clone()));
                        }
                    }
                    // 持久化缓存
                    if let Some(db) = &db {
                        if let Ok(Some(result)) = db.get_geocode_cache(lat, lng) {
                            let mut cache_guard = cache.lock().unwrap();
                            cache_guard.insert(key.clone(), result.clone());
                            return (lat, lng, Some(result));
                        }
                    }
                    let api_key = api_key.lock().unwrap().clone();
                    if api_key.is_empty() {
                        return (lat, lng, None);
                    }
                    let url = "https://restapi.amap.com/v3/geocode/regeo";
                    let resp = client
                        .get(url)
                        .query(&[
                            ("key", api_key.as_str()),
                            ("location", &format!("{},{}", lng, lat)),
                        ])
                        .send()
                        .await;
                    match resp {
                        Ok(r) => {
                            if let Ok(body) = r.json::<AmapRegeocodeResponse>().await {
                                if body.status == "1" {
                                    if let Some(regeo) = body.regeocode {
                                        let result = parse_regeocode(&regeo);
                                        let mut cache_guard = cache.lock().unwrap();
                                        cache_guard.insert(key.clone(), result.clone());
                                        if let Some(db) = &db {
                                            let _ = db.set_geocode_cache(lat, lng, &result);
                                        }
                                        return (lat, lng, Some(result));
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                    (lat, lng, None)
                }
            })
            .collect();

        let mut completed = 0;
        for task in tasks {
            if let Some(flag) = cancel_flag {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
            }
            let result = task.await;
            results.push(result);
            completed += 1;
            progress(completed, total);
        }

        results
    }

    pub async fn search_address(
        &self,
        keyword: &str,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let api_key = self.api_key.lock().unwrap().clone();
        if api_key.is_empty() {
            return Err("未配置高德API Key".into());
        }
        let url = "https://restapi.amap.com/v3/place/text";
        let resp = self
            .client
            .get(url)
            .query(&[
                ("key", &api_key),
                ("keywords", &keyword.to_string()),
                ("output", &"json".to_string()),
            ])
            .send()
            .await?;

        let body: AmapSearchResponse = resp.json().await?;

        let results = body
            .pois
            .unwrap_or_default()
            .into_iter()
            .map(|poi| {
                let (lat, lng) = parse_location(&poi.location);
                SearchResult {
                    name: poi.name,
                    address: poi.address,
                    latitude: lat,
                    longitude: lng,
                }
            })
            .collect();

        Ok(results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AmapSearchResponse {
    status: String,
    pois: Option<Vec<AmapPoi>>,
}

#[derive(Debug, Deserialize)]
struct AmapPoi {
    name: String,
    address: Option<String>,
    location: String,
}

fn parse_location(loc: &str) -> (Option<f64>, Option<f64>) {
    let parts: Vec<&str> = loc.split(',').collect();
    if parts.len() == 2 {
        let lng = parts[0].parse::<f64>().ok();
        let lat = parts[1].parse::<f64>().ok();
        (lat, lng)
    } else {
        (None, None)
    }
}
