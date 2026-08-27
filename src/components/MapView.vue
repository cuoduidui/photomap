<template>
  <div class="map-wrapper">
    <div ref="mapEl" class="map-el"></div>
    <div v-if="!mapReady" class="map-loading">
      <div class="loading-spinner"></div>
      <span>地图加载中...</span>
    </div>

    <!-- 图层切换 -->
    <div v-if="mapReady" class="layer-switch">
      <button class="layer-btn" :class="{ active: mapLayer === 'standard' }" @click="setLayer('standard')">
        <span class="layer-icon">🗺️</span>
        <span>标准</span>
      </button>
      <button class="layer-btn" :class="{ active: mapLayer === 'satellite' }" @click="setLayer('satellite')">
        <span class="layer-icon">🛰️</span>
        <span>卫星</span>
      </button>
    </div>

    <!-- 控制按钮 -->
    <div v-if="mapReady" class="map-controls">
      <button class="ctrl-btn" @click="zoomIn" title="放大">+</button>
      <button class="ctrl-btn" @click="zoomOut" title="缩小">−</button>
      <button class="ctrl-btn" @click="resetView" title="重置视图">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
          <path d="M3 3v5h5"/>
        </svg>
      </button>
    </div>

    <!-- 统计信息 -->
    <div v-if="mapReady" class="map-summary">
      <span class="summary-dot"></span>
      <span>{{ summaryText }}</span>
    </div>

    <!-- 照片弹窗 -->
    <div v-if="popupPhotos.length" class="popup-overlay" @click="closePopup">
      <div class="popup-card" @click.stop>
        <div class="popup-header">
          <span class="popup-title">
            {{ popupLocation || '' }} 
            <span class="popup-count">{{ popupPhotos.length }} 张照片</span>
          </span>
          <button class="popup-close" @click="closePopup">✕</button>
        </div>
        <div class="popup-list">
          <div v-for="p in pagedPhotos" :key="p.id" class="popup-item"
            @click="onPopupPhotoClick(p)">
            <div class="popup-item-thumb">
              <img v-if="thumbMap.get(p.id)" :src="thumbMap.get(p.id)" alt="" />
              <div v-else class="thumb-placeholder">
                <span>🖼️</span>
              </div>
            </div>
            <div class="popup-item-info">
              <div class="item-name">{{ p.file_name }}</div>
              <div class="item-meta">
                <span v-if="p.taken_time" class="item-date">{{ formatTime(p.taken_time) }}</span>
                <span v-if="p.camera_model" class="item-camera">{{ p.camera_model }}</span>
              </div>
            </div>
            <div class="popup-item-arrow">›</div>
          </div>
          <div v-if="popupPhotos.length === 0" class="popup-empty">
            暂无照片
          </div>
        </div>
        <!-- 分页 -->
        <div v-if="totalPages > 1" class="popup-pagination">
          <button class="page-btn" @click="prevPage" :disabled="currentPage === 1">
            ‹ 上一页
          </button>
          <span class="page-info">
            {{ currentPage }} / {{ totalPages }}
          </span>
          <button class="page-btn" @click="nextPage" :disabled="currentPage === totalPages">
            下一页 ›
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, onUnmounted } from "vue";
import { usePhotoStore } from "../stores/photoStore";
import { getMapBounds, getClusteredPhotos, getImageBase64, getConfig } from "../utils/tauri";

const emit = defineEmits(["photo-click", "map-click"]);
const store = usePhotoStore();
const mapEl = ref(null);
const mapReady = ref(false);
const popupPhotos = ref([]);
const popupLocation = ref("");
const currentPage = ref(1);
const PAGE_SIZE = 8;
const mapLayer = ref("standard");
const summaryText = ref("暂无已定位照片");
const thumbMap = ref(new Map());

let map = null;
let satelliteLayer = null;
let markers = [];
let amapKey = "";
let isUpdating = false;
let mapInitializing = true;
let updateTimer = null;

// 分页计算
const totalPages = computed(() => {
  return Math.ceil(popupPhotos.value.length / PAGE_SIZE);
});

const pagedPhotos = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE;
  const end = start + PAGE_SIZE;
  return popupPhotos.value.slice(start, end);
});

function prevPage() {
  if (currentPage.value > 1) {
    currentPage.value--;
    loadCurrentPageThumbs();
  }
}

function nextPage() {
  if (currentPage.value < totalPages.value) {
    currentPage.value++;
    loadCurrentPageThumbs();
  }
}

function closePopup() {
  popupPhotos.value = [];
  popupLocation.value = "";
  currentPage.value = 1;
}

function formatTime(t) {
  if (!t) return "";
  const d = new Date(t);
  if (isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

// 加载当前页的缩略图
async function loadCurrentPageThumbs() {
  const photos = pagedPhotos.value;
  for (const p of photos) {
    if (p.thumbnail_path && !thumbMap.value.has(p.id)) {
      try {
        const b64 = await getImageBase64(p.thumbnail_path);
        if (b64) {
          thumbMap.value.set(p.id, b64);
          thumbMap.value = new Map(thumbMap.value);
        }
      } catch {}
    }
  }
}

defineExpose({
  panTo,
  zoomToPhoto,
  zoomTo,
});

function zoomTo(lat, lng, zoom = 12) {
  if (!map) return;
  map.setZoomAndCenter(zoom, [lng, lat]);
}

function panTo(lat, lng, zoom = 14) {
  if (!map) return;
  map.setZoomAndCenter(zoom, [lng, lat]);
}

function zoomToPhoto(photo) {
  if (!map || !photo.latitude || !photo.longitude) return;
  map.setZoomAndCenter(15, [photo.longitude, photo.latitude]);
}

function loadAmapScript(key) {
  return new Promise((resolve, reject) => {
    if (window.AMap) {
      resolve();
      return;
    }
    const script = document.createElement("script");
    script.src = `https://webapi.amap.com/maps?v=2.0&key=${key}&plugin=AMap.Scale,AMap.ToolBar,AMap.TileLayer`;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Failed to load Amap"));
    document.head.appendChild(script);
  });
}

async function initMap() {
  try {
    amapKey = await getConfig("amap_api_key") || "";
  } catch {}

  if (!amapKey) {
    amapKey = "your_amap_key_here";
  }

  try {
    await loadAmapScript(amapKey);
  } catch (e) {
    mapEl.value.innerHTML = `<div style="display:flex;flex-direction:column;height:100%;align-items:center;justify-content:center;color:#94a3b8;font-size:0.9rem;text-align:center;padding:2rem;">
      <div style="font-size:2.5rem;margin-bottom:0.75rem;">🗺️</div>
      <div style="font-weight:600;color:#f1f5f9;margin-bottom:0.5rem;font-size:1rem;">地图加载失败</div>
      <div style="font-size:0.8rem;max-width:360px;color:#64748b;line-height:1.6;">可能原因：<br>1. 高德地图API Key未配置或无效<br>2. 网络连接问题<br>3. 请在设置中检查API Key配置</div>
      <div style="margin-top:0.75rem;font-size:0.7rem;color:#475569;">错误: ${e.message || '未知错误'}</div>
    </div>`;
    return;
  }

  map = new AMap.Map(mapEl.value, {
    zoom: 5,
    center: [104.0, 35.0],
    mapStyle: "amap://styles/normal",
    viewMode: "2D",
  });

  satelliteLayer = new AMap.TileLayer.Satellite();
  satelliteLayer.setMap(null);

  map.on("zoomend", () => {
    store.currentZoom = map.getZoom();
    scheduleUpdate();
  });

  map.on("moveend", () => {
    scheduleUpdate();
  });

  map.on("click", (e) => {
    const lng = e.lnglat.getLng();
    const lat = e.lnglat.getLat();
    emit("map-click", lat, lng);
  });

  mapReady.value = true;
  await fitBounds();
  mapInitializing = false;
  await updateMarkers();
}

function scheduleUpdate() {
  if (mapInitializing || !mapReady.value) return;
  if (updateTimer) clearTimeout(updateTimer);
  updateTimer = setTimeout(() => {
    updateTimer = null;
    updateMarkers();
  }, 200);
}

function setLayer(type) {
  mapLayer.value = type;
  if (type === "satellite") {
    satelliteLayer.setMap(map);
    map.setMapStyle("amap://styles/satellite");
  } else {
    satelliteLayer.setMap(null);
    map.setMapStyle("amap://styles/normal");
  }
}

function viewLevelText(zoom) {
  if (zoom >= 14) return "街道级别";
  if (zoom >= 11) return "市级别";
  if (zoom >= 8) return "省份级别";
  return "全国级别";
}

async function fitBounds() {
  try {
    const bounds = await getMapBounds();
    if (!bounds) {
      summaryText.value = "暂无已定位照片";
      return;
    }

    const sw = new AMap.LngLat(bounds.min_lng, bounds.min_lat);
    const ne = new AMap.LngLat(bounds.max_lng, bounds.max_lat);
    map.setBounds(new AMap.Bounds(sw, ne));
    store.currentZoom = map.getZoom();
  } catch (e) {
    console.error("[MapView] fitBounds error:", e);
  }
}

async function updateMarkers() {
  if (isUpdating) return;
  isUpdating = true;

  try {
    clearMarkers();

    const photos = store.filteredPhotos.filter((p) => p.latitude && p.longitude);
    if (photos.length === 0) {
      summaryText.value = "暂无已定位照片";
      return;
    }

    const zoom = Math.round(map.getZoom());
    const clusters = await getClusteredPhotos(zoom);

    summaryText.value = `当前视图：${viewLevelText(zoom)} · ${photos.length} 张已标记`;

    for (const cluster of clusters) {
      const marker = createMarker(cluster);
      markers.push(marker);
      marker.setMap(map);
    }
  } catch (e) {
    console.error("[MapView] updateMarkers error:", e);
  } finally {
    isUpdating = false;
  }
}

function createMarker(cluster) {
  const markerContent = document.createElement("div");
  const count = cluster.count || 1;
  let markerSize = 32;

  if (cluster.is_cluster) {
    markerSize = Math.min(56, 32 + count * 2);
    markerContent.className = "cluster-marker large";
  } else if (count > 1) {
    markerSize = 36;
    markerContent.className = "cluster-marker medium";
  } else {
    markerContent.className = "cluster-marker small";
  }

  markerContent.style.width = markerSize + "px";
  markerContent.style.height = markerSize + "px";
  markerContent.innerHTML = `<span class="cluster-count">${count}</span>`;

  const marker = new AMap.Marker({
    position: [cluster.longitude, cluster.latitude],
    content: markerContent,
    offset: new AMap.Pixel(-markerSize / 2, -markerSize / 2),
  });

  marker.on("click", async () => {
    if (cluster.is_cluster) {
      const currentZoom = map.getZoom();
      const maxZoom = 18;
      // 如果已经到最大缩放级别，直接展示照片列表
      if (currentZoom >= maxZoom - 1) {
        const photos = store.filteredPhotos.filter((p) => cluster.photo_ids.includes(p.id));
        popupPhotos.value = photos;
        popupLocation.value = `${photos.length} 张照片在此处`;
        currentPage.value = 1;
        loadCurrentPageThumbs();
      } else {
        map.setZoomAndCenter(zoomInLevel(currentZoom), [cluster.longitude, cluster.latitude]);
      }
    } else {
      const photos = store.filteredPhotos.filter((p) => cluster.photo_ids.includes(p.id));
      popupPhotos.value = photos;
      popupLocation.value = photos[0]?.city || photos[0]?.address || "";
      currentPage.value = 1;
      loadCurrentPageThumbs();
    }
  });

  return marker;
}

function zoomInLevel(current) {
  return Math.min(18, Math.floor(current) + 2);
}

function clearMarkers() {
  for (const m of markers) {
    try { m.setMap(null); } catch {}
  }
  markers = [];
}

function resetView() {
  fitBounds();
}

function zoomIn() {
  map.zoomIn();
}

function zoomOut() {
  map.zoomOut();
}

function onPopupPhotoClick(photo) {
  popupPhotos.value = [];
  emit("photo-click", photo);
}

watch(() => store.filteredPhotos, () => {
  if (mapReady.value && !mapInitializing) {
    scheduleUpdate();
  }
});

onMounted(() => {
  initMap();
});

onUnmounted(() => {
  if (updateTimer) {
    clearTimeout(updateTimer);
    updateTimer = null;
  }
  clearMarkers();
  if (satelliteLayer) {
    satelliteLayer.setMap(null);
    satelliteLayer = null;
  }
  if (map) {
    map.destroy();
    map = null;
  }
});
</script>

<style scoped>
.map-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
}

.map-el {
  width: 100%;
  height: 100%;
}

.map-loading {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--muted);
  font-size: 0.9rem;
}
.loading-spinner {
  width: 28px;
  height: 28px;
  border: 3px solid var(--rule);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.layer-switch {
  position: absolute;
  top: 16px;
  right: 16px;
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  z-index: 10;
}
.layer-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 14px;
  border-radius: 6px;
  font-size: 0.7rem;
  color: var(--muted);
  transition: all 0.2s ease;
}
.layer-btn:hover {
  background: var(--accent-light);
  color: var(--ink);
}
.layer-btn.active {
  background: var(--accent-light);
  color: var(--accent);
}
.layer-icon { font-size: 1.1rem; }

.map-controls {
  position: absolute;
  right: 16px;
  bottom: 24px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px;
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  z-index: 10;
}
.ctrl-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  color: var(--muted);
  border-radius: 6px;
  transition: all 0.15s ease;
}
.ctrl-btn:hover {
  background: var(--accent-light);
  color: var(--accent);
}

.map-summary {
  position: absolute;
  top: 16px;
  left: 16px;
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg2);
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 0.78rem;
  color: var(--muted);
  border: 1px solid var(--rule);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  z-index: 10;
}
.summary-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 6px rgba(16, 172, 132, 0.5);
}

.popup-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  animation: fadeIn 0.2s ease;
}
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.popup-card {
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 12px;
  padding: 0;
  width: 360px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
  animation: slideUp 0.25s ease;
  overflow: hidden;
}
@keyframes slideUp {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}

.popup-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--rule);
  background: var(--bg);
  flex-shrink: 0;
}
.popup-title {
  font-weight: 600;
  font-size: 0.85rem;
  color: var(--ink);
  display: flex;
  align-items: center;
  gap: 8px;
}
.popup-count {
  font-size: 0.7rem;
  color: var(--muted);
  font-weight: 400;
  padding: 2px 8px;
  background: var(--accent-light);
  border-radius: 10px;
}
.popup-close {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--accent-light);
  color: var(--muted);
  font-size: 0.7rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.popup-close:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.popup-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.popup-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
}
.popup-item:hover {
  background: var(--accent-light);
  border-color: var(--accent-light);
}
.popup-item-thumb {
  width: 48px;
  height: 48px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg);
}
.popup-item-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  color: var(--muted);
  background: var(--bg);
}
.popup-item-info {
  flex: 1;
  min-width: 0;
}
.item-name {
  font-size: 0.78rem;
  color: var(--ink);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.item-meta {
  margin-top: 2px;
  display: flex;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--muted);
}
.item-date, .item-camera {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.popup-item-arrow {
  color: var(--muted);
  font-size: 1rem;
  flex-shrink: 0;
}
.popup-empty {
  padding: 30px;
  text-align: center;
  color: var(--muted);
  font-size: 0.8rem;
}

.popup-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-top: 1px solid var(--rule);
  background: var(--bg);
  flex-shrink: 0;
}
.page-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.72rem;
  color: var(--muted);
  background: var(--accent-light);
  transition: all 0.15s ease;
}
.page-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.page-info {
  font-size: 0.72rem;
  color: var(--muted);
}

:deep(.cluster-marker) {
  background: #0a84d0;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  border: 3px solid #fff;
  box-shadow: 0 4px 16px rgba(10, 132, 208, 0.3);
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}
:deep(.cluster-marker.small) {
  font-size: 0.75rem;
  box-shadow: 0 2px 8px rgba(10, 132, 208, 0.2);
}
:deep(.cluster-marker.medium) {
  font-size: 0.85rem;
}
:deep(.cluster-marker.large) {
  font-size: 0.95rem;
  background: #00b2a9;
  box-shadow: 0 6px 20px rgba(0, 178, 169, 0.3);
}
:deep(.cluster-marker:hover) {
  transform: scale(1.15);
  box-shadow: 0 6px 24px var(--accent-light);
}
:deep(.cluster-count) {
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  line-height: 1;
}
</style>
