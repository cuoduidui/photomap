<template>
  <div class="map-wrapper">
    <div ref="mapEl" class="map-el"></div>
    <div v-if="!mapReady" class="map-loading">
      <div class="loading-spinner"></div>
      <span>{{ $t("map.loading") }}</span>
    </div>

    <!-- 图层切换 -->
    <div v-if="mapReady" class="layer-switch">
      <button class="layer-btn" :class="{ active: mapLayer === 'standard' }" @click="setLayer('standard')">
        <span class="layer-icon">🗺️</span>
        <span>{{ $t("map.standard") }}</span>
      </button>
      <button class="layer-btn" :class="{ active: mapLayer === 'satellite' }" @click="setLayer('satellite')">
        <span class="layer-icon">🛰️</span>
        <span>{{ $t("map.satellite") }}</span>
      </button>
    </div>

    <!-- 控制按钮 -->
    <div v-if="mapReady" class="map-controls">
      <button class="ctrl-btn" @click="zoomIn" :title="$t('map.zoomIn')">+</button>
      <button class="ctrl-btn" @click="zoomOut" :title="$t('map.zoomOut')">−</button>
      <button class="ctrl-btn" @click="resetView" :title="$t('map.resetView')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
          <path d="M3 3v5h5"/>
        </svg>
      </button>
    </div>

    <!-- 路线回放入口 -->
    <div v-if="mapReady" class="route-entry">
      <button class="route-btn" :class="{ active: routeLine }" @click="toggleRoutePlay"
        :title="$t('map.routeReplayTitle')">
        <span>{{ routeLine ? $t('map.stopReplay') : $t('map.startReplay') }}</span>
      </button>
    </div>

    <!-- 路线回放控制条 -->
    <div v-if="routeLine" class="route-player" @click.stop>
      <div class="route-thumb">
        <img v-if="routeThumb" :src="routeThumb" alt="" />
        <div v-else class="route-thumb-ph">🖼️</div>
      </div>
      <div class="route-info">
        <div class="route-name" :title="currentRoutePhoto ? currentRoutePhoto.file_name : ''">
          {{ currentRoutePhoto ? currentRoutePhoto.file_name : '' }}
        </div>
        <div class="route-meta">
          <span v-if="currentRoutePhoto && currentRoutePhoto.taken_time">{{ formatTime(currentRoutePhoto.taken_time) }}</span>
          <span v-if="currentRoutePhoto && currentRoutePhoto.address" class="route-addr">{{ currentRoutePhoto.address }}</span>
        </div>
        <div class="route-progress">
          <div class="route-progress-bar">
            <div class="route-progress-fill" :style="{ width: routeProgressPct + '%' }"></div>
          </div>
          <span class="route-step">{{ routeStepText }}</span>
        </div>
      </div>
      <div class="route-controls">
        <button class="route-ctrl" @click="togglePause" :title="routePaused ? $t('map.resume') : $t('map.pause')">
          {{ routePaused ? '▶' : (routeFinished ? '↻' : '⏸') }}
        </button>
        <button class="route-ctrl" @click="stopRoute" :title="$t('map.stopClear')">⏹</button>
        <select class="route-speed" v-model="routeSpeed" :title="$t('map.playSpeed')">
          <option :value="2400">0.5×</option>
          <option :value="1200">1×</option>
          <option :value="600">2×</option>
          <option :value="300">4×</option>
        </select>
      </div>
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
            <span class="popup-count">{{ $t("map.photosHere", { n: popupPhotos.length }) }}</span>
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
            {{ $t("map.noPhotos") }}
          </div>
        </div>
        <!-- 分页 -->
        <div v-if="totalPages > 1" class="popup-pagination">
          <button class="page-btn" @click="prevPage" :disabled="currentPage === 1">
            {{ $t("map.prevPage") }}
          </button>
          <span class="page-info">
            {{ currentPage }} / {{ totalPages }}
          </span>
          <button class="page-btn" @click="nextPage" :disabled="currentPage === totalPages">
            {{ $t("map.nextPage") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { getMapBounds, getClusteredPhotos, getImageBase64, getConfig } from "../utils/tauri";
import { toAmapLngLat } from "../utils/geo";

const emit = defineEmits(["photo-click", "map-click"]);
const store = usePhotoStore();
const { t } = useI18n();
const mapEl = ref(null);
const mapReady = ref(false);
const popupPhotos = ref([]);
const popupLocation = ref("");
const currentPage = ref(1);
const PAGE_SIZE = 8;
const mapLayer = ref("standard");
// 摘要文案改为「状态 + computed」，语言切换时自动重新翻译
const summaryState = ref({ type: "view", zoom: 3, n: 0 });
function setSummary(type, zoom, n) {
  summaryState.value = { type, zoom: zoom || 3, n: n || 0 };
}
const summaryText = computed(() => {
  const s = summaryState.value;
  if (s.type === "needMore") return t("map.routeNeedMore");
  if (s.type === "finished") return t("map.routeFinished");
  if (s.type === "none") return t("map.noLocatedPhotos");
  return t("map.summaryView", { level: viewLevelText(s.zoom), n: s.n });
});
const thumbMap = ref(new Map());

let map = null;
let satelliteLayer = null;
let markers = [];
let amapKey = "";
let isUpdating = false;
let mapInitializing = true;
let updateTimer = null;

// 路线回放状态
let routeLine = null;
let startMarker = null;
let endMarker = null;
let carMarker = null;
let routeTimer = null;
let routePhotos = [];
let routeSeg = 0;
let routeProgress = 0;
let routeThumbToken = 0;
const routePlaying = ref(false);
const routePaused = ref(false);
const routeFinished = ref(false);
const routeSpeed = ref(1200);
const routeThumb = ref("");

// 分页计算
const totalPages = computed(() => {
  return Math.ceil(popupPhotos.value.length / PAGE_SIZE);
});

const pagedPhotos = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE;
  const end = start + PAGE_SIZE;
  return popupPhotos.value.slice(start, end);
});

const currentRoutePhoto = computed(() => {
  if (!routeLine || routePhotos.length === 0) return null;
  const idx = Math.max(0, Math.min(routePhotos.length - 1, Math.round(routeSeg + routeProgress)));
  return routePhotos[idx];
});

const routeProgressPct = computed(() => {
  if (routePhotos.length < 2) return 0;
  return ((routeSeg + routeProgress) / (routePhotos.length - 1)) * 100;
});

const routeStepText = computed(() => {
  if (routePhotos.length === 0) return "0 / 0";
  const cur = Math.max(1, Math.min(routePhotos.length, Math.round(routeSeg + routeProgress) + 1));
  return `${cur} / ${routePhotos.length}`;
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

// ---------- 路线回放 ----------
function toggleRoutePlay() {
  if (routeLine) {
    stopRoute();
  } else {
    startRoute();
  }
}

function startRoute() {
  stopRoute();
  routePhotos = store.filteredPhotos
    .filter((p) => p.latitude && p.longitude)
    .sort((a, b) => {
      const ta = a.taken_time || "9999-12-31 23:59:59";
      const tb = b.taken_time || "9999-12-31 23:59:59";
      if (ta !== tb) return ta < tb ? -1 : 1;
      return (a.id || "").localeCompare(b.id || "");
    });

  if (routePhotos.length < 2) {
    setSummary("needMore");
    return;
  }

  drawRouteLine();
  fitRouteBounds();

  routeSeg = 0;
  routeProgress = 0;
  routeFinished.value = false;
  routePaused.value = false;
  routePlaying.value = true;
  placeCar(routePhotos[0].latitude, routePhotos[0].longitude);
  startTick();
  loadRouteThumb(routePhotos[0]);
}

function drawRouteLine() {
  const path = routePhotos.map((p) => toAmapLngLat(p.latitude, p.longitude));
  routeLine = new AMap.Polyline({
    path,
    strokeColor: "#0ea5e9",
    strokeWeight: 5,
    strokeOpacity: 0.85,
    lineJoin: "round",
    lineCap: "round",
  });
  routeLine.setMap(map);

  const startEl = document.createElement("div");
  startEl.style.cssText = "width:22px;height:22px;border-radius:50%;background:#22c55e;color:#fff;font-size:11px;font-weight:700;display:flex;align-items:center;justify-content:center;border:2px solid #fff;box-shadow:0 1px 5px rgba(0,0,0,.35);";
  startEl.textContent = t("map.startLabel");
  startMarker = new AMap.Marker({
    position: path[0],
    content: startEl,
    offset: new AMap.Pixel(-11, -11),
  });
  startMarker.setMap(map);

  const endEl = document.createElement("div");
  endEl.style.cssText = "width:22px;height:22px;border-radius:50%;background:#ef4444;color:#fff;font-size:11px;font-weight:700;display:flex;align-items:center;justify-content:center;border:2px solid #fff;box-shadow:0 1px 5px rgba(0,0,0,.35);";
  endEl.textContent = t("map.endLabel");
  endMarker = new AMap.Marker({
    position: path[path.length - 1],
    content: endEl,
    offset: new AMap.Pixel(-11, -11),
  });
  endMarker.setMap(map);

  const carEl = document.createElement("div");
  carEl.style.cssText = "width:26px;height:26px;border-radius:50%;background:#0ea5e9;font-size:15px;display:flex;align-items:center;justify-content:center;border:2px solid #fff;box-shadow:0 2px 8px rgba(14,165,233,.55);";
  carEl.textContent = "🚗";
  carMarker = new AMap.Marker({
    position: path[0],
    content: carEl,
    offset: new AMap.Pixel(-13, -13),
  });
  carMarker.setMap(map);
}

function placeCar(lat, lng) {
  if (carMarker) carMarker.setPosition(toAmapLngLat(lat, lng));
}

function fitRouteBounds() {
  if (!map || routePhotos.length === 0) return;
  let minLat = Infinity, maxLat = -Infinity, minLng = Infinity, maxLng = -Infinity;
  for (const p of routePhotos) {
    minLat = Math.min(minLat, p.latitude);
    maxLat = Math.max(maxLat, p.latitude);
    minLng = Math.min(minLng, p.longitude);
    maxLng = Math.max(maxLng, p.longitude);
  }
  const pad = 0.01;
  const sw = toAmapLngLat(minLat - pad, minLng - pad);
  const ne = toAmapLngLat(maxLat + pad, maxLng + pad);
  map.setBounds(new AMap.Bounds(
    new AMap.LngLat(sw[0], sw[1]),
    new AMap.LngLat(ne[0], ne[1])
  ));
}

function startTick() {
  if (routeTimer) clearInterval(routeTimer);
  routeTimer = setInterval(tick, 50);
}

function tick() {
  if (!routePlaying.value || routePaused.value || routePhotos.length < 2) return;
  if (routeSeg >= routePhotos.length - 1) {
    finishRoute();
    return;
  }

  routeProgress += 50 / routeSpeed.value;
  if (routeProgress >= 1) {
    routeProgress = 0;
    routeSeg++;
    if (routeSeg >= routePhotos.length - 1) {
      finishRoute();
      return;
    }
    const p = routePhotos[routeSeg];
    placeCar(p.latitude, p.longitude);
    if (map) map.panTo(toAmapLngLat(p.latitude, p.longitude));
    loadRouteThumb(p);
  } else {
    const a = routePhotos[routeSeg];
    const b = routePhotos[routeSeg + 1];
    const lat = a.latitude + (b.latitude - a.latitude) * routeProgress;
    const lng = a.longitude + (b.longitude - a.longitude) * routeProgress;
    placeCar(lat, lng);
  }
}

function finishRoute() {
  routePlaying.value = false;
  routeFinished.value = true;
  if (routeTimer) {
    clearInterval(routeTimer);
    routeTimer = null;
  }
  if (routePhotos.length) {
    const last = routePhotos[routePhotos.length - 1];
    placeCar(last.latitude, last.longitude);
    loadRouteThumb(last);
  }
  setSummary("finished");
}

function togglePause() {
  if (!routeLine) return;
  if (routeFinished.value) {
    routeSeg = 0;
    routeProgress = 0;
    routeFinished.value = false;
    routePaused.value = false;
    routePlaying.value = true;
    if (routePhotos.length) {
      const first = routePhotos[0];
      placeCar(first.latitude, first.longitude);
      loadRouteThumb(first);
    }
    startTick();
    return;
  }
  routePaused.value = !routePaused.value;
  routePlaying.value = !routePaused.value;
  if (routePlaying.value && !routeTimer) startTick();
}

function stopRoute() {
  if (routeTimer) {
    clearInterval(routeTimer);
    routeTimer = null;
  }
  for (const obj of [routeLine, startMarker, endMarker, carMarker]) {
    try { if (obj) obj.setMap(null); } catch {}
  }
  routeLine = null;
  startMarker = null;
  endMarker = null;
  carMarker = null;
  routePhotos = [];
  routeSeg = 0;
  routeProgress = 0;
  routePlaying.value = false;
  routePaused.value = false;
  routeFinished.value = false;
  routeThumb.value = "";
  routeThumbToken++;
}

async function loadRouteThumb(photo) {
  const token = ++routeThumbToken;
  routeThumb.value = "";
  if (!photo || !photo.thumbnail_path) return;
  try {
    const b64 = await getImageBase64(photo.thumbnail_path);
    if (token === routeThumbToken && b64) {
      routeThumb.value = b64;
    }
  } catch {}
}

watch(currentRoutePhoto, (p) => {
  if (p && routePlaying.value) loadRouteThumb(p);
});

defineExpose({
  panTo,
  zoomToPhoto,
  zoomTo,
});

function zoomTo(lat, lng, zoom = 12) {
  if (!map) return;
  map.setZoomAndCenter(zoom, toAmapLngLat(lat, lng));
}

function panTo(lat, lng, zoom = 14) {
  if (!map) return;
  map.setZoomAndCenter(zoom, toAmapLngLat(lat, lng));
}

function zoomToPhoto(photo) {
  if (!map || !photo.latitude || !photo.longitude) return;
  map.setZoomAndCenter(15, toAmapLngLat(photo.latitude, photo.longitude));
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
      <div style="font-weight:600;color:#f1f5f9;margin-bottom:0.5rem;font-size:1rem;">${t("map.loadFailedTitle")}</div>
      <div style="font-size:0.8rem;max-width:360px;color:#64748b;line-height:1.6;">${t("map.loadFailedReasons")}</div>
      <div style="margin-top:0.75rem;font-size:0.7rem;color:#475569;">${t("map.error", { error: e.message || t("map.unknownError") })}</div>
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
  if (zoom >= 14) return t("map.viewStreet");
  if (zoom >= 11) return t("map.viewCity");
  if (zoom >= 8) return t("map.viewProvince");
  return t("map.viewCountry");
}

async function fitBounds() {
  try {
    const bounds = await getMapBounds();
    if (!bounds) {
      setSummary("none");
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
      setSummary("none");
      return;
    }
    const filteredIdSet = new Set(photos.map((p) => p.id));

    const zoom = Math.round(map.getZoom());
    const clusters = await getClusteredPhotos(zoom);

    setSummary("view", zoom, photos.length);

    for (const cluster of clusters) {
      // 标记必须遵循当前筛选：只显示包含被筛选照片的聚类
      const filteredIds = (cluster.photo_ids || []).filter((id) => filteredIdSet.has(id));
      if (filteredIds.length === 0) {
        continue;
      }
      // 标记数量用筛选后的照片数，保证与点开列表数量一致
      const marker = createMarker(cluster, filteredIds.length);
      markers.push(marker);
      marker.setMap(map);
    }
  } catch (e) {
    console.error("[MapView] updateMarkers error:", e);
  } finally {
    isUpdating = false;
  }
}

function createMarker(cluster, filteredCount) {
  const markerContent = document.createElement("div");
  const count = filteredCount != null ? filteredCount : (cluster.count || 1);
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
        popupLocation.value = photos[0]?.address || photos[0]?.city || t("map.photosAtLocation", { n: photos.length });
        currentPage.value = 1;
        loadCurrentPageThumbs();
      } else {
        map.setZoomAndCenter(zoomInLevel(currentZoom), [cluster.longitude, cluster.latitude]);
      }
    } else {
      const photos = store.filteredPhotos.filter((p) => cluster.photo_ids.includes(p.id));
      popupPhotos.value = photos;
      popupLocation.value = photos[0]?.address || photos[0]?.city || photos[0]?.province || "";
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
  // 筛选变化时停止路线回放，避免路线与标记不一致
  if (routeLine) stopRoute();
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
  stopRoute();
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

.route-entry {
  position: absolute;
  bottom: 24px;
  left: 16px;
  z-index: 10;
}
.route-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 9px 14px;
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 8px;
  font-size: 0.78rem;
  color: var(--muted);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  transition: all 0.2s ease;
}
.route-btn:hover {
  background: var(--accent-light);
  color: var(--accent);
}
.route-btn.active {
  background: rgba(239, 68, 68, 0.12);
  border-color: rgba(239, 68, 68, 0.35);
  color: #ef4444;
}

.route-player {
  position: absolute;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 12px;
  width: min(520px, calc(100% - 32px));
  padding: 10px 14px;
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 12px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
  z-index: 11;
}
.route-thumb {
  width: 46px;
  height: 46px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-card);
  display: flex;
  align-items: center;
  justify-content: center;
}
.route-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.route-thumb-ph {
  font-size: 1.3rem;
  opacity: 0.5;
}
.route-info {
  flex: 1;
  min-width: 0;
}
.route-name {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.route-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
  font-size: 0.68rem;
  color: var(--text-muted);
}
.route-addr {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.route-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
}
.route-progress-bar {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: var(--border);
  overflow: hidden;
}
.route-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-2));
  border-radius: 2px;
  transition: width 0.1s linear;
}
.route-step {
  font-size: 0.66rem;
  color: var(--text-muted);
  flex-shrink: 0;
}
.route-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.route-ctrl {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.9rem;
  color: var(--text-secondary);
  background: var(--bg-card);
  border: 1px solid var(--rule);
  border-radius: 8px;
  transition: all 0.15s ease;
}
.route-ctrl:hover {
  color: var(--accent);
  border-color: var(--accent);
}
.route-speed {
  height: 32px;
  padding: 0 6px;
  font-size: 0.72rem;
  color: var(--text-secondary);
  background: var(--bg-card);
  border: 1px solid var(--rule);
  border-radius: 8px;
  outline: none;
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
