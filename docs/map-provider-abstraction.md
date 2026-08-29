# 地图 Provider 抽象设计文档

> 目标：让 PhotoMap 不再绑定高德地图，国内用户继续用高德，海外用户自动切换到 OpenStreetMap / MapTiler / Mapbox 等可用的地图源。

## 1. 背景与问题

PhotoMap 目前的地图能力（瓦片显示、逆地理编码、地址解析、地点树）全部依赖高德地图（AMap）：

- 地图 JS API（webapi.amap.com）与 Web 服务 API（restapi.amap.com）服务器位于中国境内
- 高德地图数据以中国境内为主，海外区域只有大城市基础底图，道路与 POI 稀疏
- 逆地理编码始终返回中文地址，与英文/日文等界面语言不匹配
- 高德坐标系为 GCJ-02，海外地图（OSM 等）使用 WGS-84，坐标换算逻辑目前散落在前端与后端
- 高德 Key 要求「安全密钥 + 域名白名单」，Tauri 发布版 origin（tauri://localhost）存在白名单配置风险

结论：**当前架构无法支撑海外用户正常使用**，需要把地图相关能力抽象为可插拔的 Provider。

## 2. 目标与非目标

### 目标

- 前端地图渲染支持多 Provider，切换不影响照片标记、聚类、路线回放等核心功能
- 后端地理编码支持多 Provider，统一地址数据结构
- 设置页提供「地图源」选择与各 Provider 的 Key 配置
- 默认规则：界面语言为简体中文时用高德，其他语言自动用海外 Provider（可在设置中覆盖）
- 所有改动保持向后兼容，高德路径行为不变

### 非目标（本期不做）

- 不引入服务端代理（所有请求仍由客户端直连）
- 不做离线地图
- 不迁移既有照片数据（坐标始终以 WGS-84 为唯一存储标准）

## 3. 现状分析

### 3.1 前端地图（src/components/MapView.vue）

MapView.vue 与 AMap API 深度耦合：

- 通过 `document.createElement("script")` 动态加载 `webapi.amap.com/maps?v=2.0&key=...`
- 直接使用 `window.AMap`、`new AMap.Map`、`AMap.Marker`、`AMap.Polyline`、`AMap.Bounds`、`AMap.LngLat`、`AMap.Pixel`
- 坐标显示统一走 `utils/geo.js` 的 `toAmapLngLat()`（WGS-84 → GCJ-02）
- 卫星图层、路线回放（车辆标记沿折线移动）、聚合标记交互均依赖 AMap 具体 API

### 3.2 坐标转换

- 前端：`src/utils/geo.js`（wgs84ToGcj02 / toAmapLngLat）
- 后端：`src-tauri/src/coord.rs`
- 照片 EXIF 坐标为 WGS-84，仅渲染给高德时才转 GCJ-02

### 3.3 聚类（可复用）

`src-tauri/src/cluster.rs` 基于 WGS-84 坐标做网格聚类（`cluster_photos(zoom)`），**与具体地图 SDK 无关**，可直接复用。

### 3.4 地理编码（后端，仅高德）

`src-tauri/src/geocode.rs`：

- 逆地理编码：restapi.amap.com/v3/geocode/regeo
- 地点搜索：restapi.amap.com/v3/place/text
- 返回结构为高德字段（province/city/district/adcode 等），与地点树强耦合

### 3.5 地点树（中国地址模型）

`src/components/LocationList.vue` 与 `photoStore.treeByLocation`：

- 层级固定为「省 → 市 → 区县 → 街道 → 地址」
- 内置直辖市表（MUNICIPALITIES）、中文街道正则（STREET_RE）
- 海外地址没有省市区概念，需要新的层级模型

## 4. 目标架构

### 4.1 前端：MapProvider 接口

新建 `src/maps/` 目录：

```
src/maps/
├── index.js           # Provider 注册表 + 当前 Provider 选择
├── base.js            # MapProvider 抽象接口
├── amap.js            # 高德实现（从 MapView 抽取）
├── osm.js             # OpenStreetMap 实现（Leaflet 或 MapLibre）
└── maptiler.js        # MapTiler 实现（可选，后续）
```

抽象接口（建议）：

```js
class MapProvider {
  id;                    // "amap" | "osm" | "maptiler"
  displayNameKey;        // i18n key
  coordinateMode;        // "gcj02" | "wgs84"
  requiresKey;           // 是否需要用户配置 Key

  loadScript(settings);  // 动态加载 SDK，返回 Promise
  createMap(el, opts);   // 创建地图实例
  destroy();

  // 标记
  addMarker(pos, opts);       // pos 为 { lat, lng }（WGS-84，Provider 内部转换）
  clearMarkers();
  setClusterMarkers(clusters); // 聚类标记（复用后端聚类结果）

  // 视图
  setView(center, zoom);
  panTo(lat, lng);
  fitBounds(bounds);
  setSatellite(enabled);
  getZoom();

  // 路线回放
  drawPolyline(points);
  clearRoute();
  moveVehicle(lat, lng);     // 车辆标记

  // 事件
  on(event, handler);        // click / zoomend / moveend 等
}
```

坐标约定：**接口层一律使用 WGS-84**，Provider 内部按 `coordinateMode` 决定是否做 GCJ-02 转换（amap 转、osm 不转），这样上层组件完全无感。

### 4.2 后端：Geocoder 接口

`src-tauri/src/geocode.rs` 重构为 trait + 实现：

```rust
pub struct GeoAddress {
    pub country: Option<String>,
    pub province: Option<String>,   // 中国：省；海外：州/邦
    pub city: Option<String>,
    pub district: Option<String>,
    pub street: Option<String>,
    pub address: Option<String>,    // 完整地址原文
    pub raw: serde_json::Value,     // Provider 原始返回（调试/扩展用）
}

pub trait Geocoder: Send + Sync {
    fn reverse(&self, lat: f64, lng: f64) -> Result<GeoAddress, String>;
    fn search(&self, keyword: &str) -> Result<Vec<GeoAddress>, String>;
}
```

实现：

- `AmapGeocoder`：现有逻辑，地址字段映射到 GeoAddress
- `NominatimGeocoder`：OSM 官方 Nominatim（免费，需遵守使用政策与 User-Agent）
- `MapTilerGeocoder`：MapTiler Geocoding API（免费额度，推荐作为海外默认）

按 `map_provider` 配置项路由，Key 走现有 DPAPI 加密存储。

### 4.3 地点树：统一节点模型

把「省 → 市 → 区县 → 街道 → 地址」从硬编码改为数据驱动：

- 中国地址：country=中国 → province → city → district → street → address
- 海外地址：country → state → city → address（无 street 时地址直接挂城市下）
- `LocationList.vue` 与 `treeByLocation` 改为按统一层级字段构建，中国相关正则（直辖市表、街道正则）仅在中国地址分支生效

### 4.4 设置页

设置新增「地图源」区块：

- 下拉选择：自动（按语言）/ 高德 / OpenStreetMap / MapTiler
- 各 Provider 独立 Key 输入框（高德 Key、MapTiler Key），OSM 无需 Key
- 切换后立即重载地图，无需重启

## 5. 关键技术点与风险

| 问题 | 方案 | 风险/注意 |
|---|---|---|
| GCJ-02 偏移 | Provider 内做坐标转换，接口统一 WGS-84 | 转换仅限中国境内，海外坐标不变 |
| 卫星图 | OSM 无官方卫星源，MapTiler 有；卫星按钮按 Provider 能力显示/隐藏 | 功能降级需在 UI 上体现 |
| Nominatim 使用政策 | 海外默认优先 MapTiler；Nominatim 仅作为免费备选 | 高频请求会被限流，需做本地缓存 |
| 聚合标记 | 复用后端聚类结果，前端按 Provider 渲染 | 不同 SDK 的聚合样式差异需统一视觉 |
| CSP | tauri.conf.json 需为每个 Provider 追加 script/connect 域名 | 上线前逐 Provider 验证 |
| Key 安全 | 继续走 DPAPI 加密 + 用户自填 | 与现有机制一致 |
| 逆地理编码地址语言 | Nominatim 可指定 accept-language 跟随界面语言 | 高德始终中文，海外地址显示为当地语言 |

## 6. 实施步骤

### Phase 1：接口抽取（不改变行为）

- 新建 `src/maps/`，把 MapView.vue 中 AMap 相关逻辑原样抽到 `amap.js`
- MapView.vue 改为通过 Provider 接口调用，行为零变化
- 后端定义 `Geocoder` trait，`AmapGeocoder` 包装现有实现
- 设置页增加 Provider 注册表（此时只有 amap 一个选项，UI 可先隐藏）

### Phase 2：海外可用（核心价值）

- 实现 `osm.js`（建议 Leaflet，体量小、文档全；或 MapLibre GL 支持矢量瓦片）
- 实现 `NominatimGeocoder` / `MapTilerGeocoder`
- 地点树支持海外层级
- 默认规则：界面语言非简体中文时自动切到 OSM/MapTiler
- 设置页开放 Provider 选择与 Key 配置

### Phase 3：体验完善

- 卫星图层能力探测与降级提示
- 逆地理编码结果本地缓存（SQLite 表）
- 聚合标记视觉统一
- 多 Provider 截图与回归测试

### Phase 4：文档与发布

- 更新 README 多地图源说明
- 每个 Provider 的申请与配置指引

## 7. 验收标准

- 简体中文界面下行为与现状完全一致（高德）
- 英文/日文等界面在无高德 Key 时，地图、标记、路线回放、地点树、地址识别全部可用
- 切换 Provider 后无需重启应用
- 聚类数量、筛选联动、跳转定位在两种 Provider 下表现一致
- 海外照片逆地理编码返回当地语言地址，地点树层级正确

## 8. 相关文件清单

| 文件 | 改动类型 |
|---|---|
| src/components/MapView.vue | 重构：AMap 逻辑迁出 |
| src/utils/geo.js | 保留，供 amap provider 使用 |
| src-tauri/src/coord.rs | 保留 |
| src-tauri/src/geocode.rs | 重构为 trait + 多实现 |
| src-tauri/src/cluster.rs | 不动（已与 Provider 无关） |
| src/components/LocationList.vue | 地点树支持海外层级 |
| src/stores/photoStore.js | treeByLocation 统一层级模型 |
| src/components/SettingsDialog.vue | 地图源选择与 Key 配置 |
| src/i18n/locales/*.js | 新增 provider 相关文案 |
| src-tauri/tauri.conf.json | CSP 追加 Provider 域名 |

---

## English Summary

PhotoMap currently depends entirely on AMap (China-only). This document proposes a pluggable **Map Provider** architecture:

- Frontend: a `MapProvider` interface (`src/maps/`) with AMap, OpenStreetMap (Leaflet/MapLibre) and MapTiler implementations. All APIs take WGS-84 coordinates; each provider handles its own coordinate system internally (AMap converts to GCJ-02).
- Backend: a `Geocoder` trait with `AmapGeocoder`, `NominatimGeocoder` and `MapTilerGeocoder`, returning a unified `GeoAddress` model.
- Location tree: switch from a hard-coded Chinese hierarchy (province → city → district → street) to a data-driven model supporting both Chinese and overseas addresses.
- Settings: a "map source" selector with per-provider API keys.
- Default rule: use AMap when the UI language is Simplified Chinese, otherwise use an overseas provider (overridable).

Implementation is split into 4 phases: interface extraction (no behavior change) → overseas providers (core value) → UX polish (satellite fallback, geocode caching, marker styling) → docs and release.
