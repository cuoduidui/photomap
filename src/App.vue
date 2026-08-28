<template>
  <div class="app-container">
    <!-- 顶部工具栏 -->
    <header class="toolbar">
      <div class="toolbar-left">
        <span class="app-logo">PhotoMap</span>
      </div>
      <div class="toolbar-center">
        <button class="btn-primary" @click="triggerImportFolder" :disabled="store.loading">
          {{ store.loading ? $t("common.loading") : $t("app.importFolder") }}
        </button>
        <button class="btn-secondary" @click="triggerImportFiles" :disabled="store.loading"> {{ $t("app.selectImages") }} </button>
        <button class="btn-secondary" @click="showFilterBar = !showFilterBar"> {{ $t("app.filter") }} </button>
        <button class="btn-secondary" @click="runBatchGeocode" :disabled="!store.apiConfigured || store.geocodeProgress">
          {{ $t("app.reverseGeocode") }}
        </button>
        <button class="btn-ghost" @click="showSettings = true">{{ $t("app.settings") }}</button>
        <div class="theme-popover-wrap">
          <button class="btn-ghost" :title="$t('app.skin')" @click="showThemePopover = !showThemePopover">🎨</button>
          <div v-if="showThemePopover" class="theme-popover" @click.stop>
            <div class="theme-popover-title">{{ $t("app.oneClickTheme") }}</div>
            <ThemeSwitcher :current="currentTheme" inline @select="onSelectTheme" />
          </div>
        </div>
      </div>
      <div class="toolbar-right">
        <button class="btn-ghost"
          @click="store.selectedPhotoIds.length ? (showLocationDialog = true) : null"
          :disabled="!store.selectedPhotoIds.length">
          {{ $t("app.annotateLocation") }} ({{ store.selectedPhotoIds.length }})
        </button>
      </div>
    </header>

    <!-- 导入进度 -->
    <div v-if="store.importProgress" class="progress-container">
      <span v-if="store.importProgress.scanning">{{ $t("app.scanningFolder", { n: store.importProgress.total }) }}</span>
      <span v-else>{{ $t("app.importingPhotos", { done: store.importProgress.done, total: store.importProgress.total }) }}</span>
      <div class="progress-bar">
        <div class="fill" :style="{ width: importProgressPercent + '%' }" :class="{ indeterminate: store.importProgress.scanning }" />
      </div>
      <button class="cancel-btn" @click="onCancelLongTask">{{ $t("common.cancel") }}</button>
    </div>
    <!-- 逆地理编码进度 -->
    <div v-else-if="store.geocodeProgress" class="progress-container">
      <span>{{ $t("app.geocoding", { done: store.geocodeProgress.done, total: store.geocodeProgress.total }) }}</span>
      <div class="progress-bar">
        <div class="fill" :style="{ width: geocodeProgressPercent + '%' }" />
      </div>
      <button class="cancel-btn" @click="onCancelLongTask">{{ $t("common.cancel") }}</button>
    </div>

    <!-- 筛选器 -->
    <FilterBar v-if="showFilterBar" @close="showFilterBar = false" />

    <!-- 主体区域 -->
    <div class="main-body">
      <!-- 左侧可拖拽侧边栏 -->
      <aside class="sidebar" :style="{ width: sidebarWidth + 'px' }">
        <div class="sidebar-tabs">
          <button :class="{ active: activeTab === 'location' }" @click="activeTab = 'location'">{{ $t("app.tabs.location") }}</button>
          <button :class="{ active: activeTab === 'timeline' }" @click="activeTab = 'timeline'">{{ $t("app.tabs.timeline") }}</button>
          <button :class="{ active: activeTab === 'photos' }" @click="activeTab = 'photos'">{{ $t("app.tabs.photos") }}</button>
          <button :class="{ active: activeTab === 'album' }" @click="activeTab = 'album'">{{ $t("app.tabs.album") }}</button>
          <button :class="{ active: activeTab === 'trip' }" @click="activeTab = 'trip'">{{ $t("app.tabs.trip") }}</button>
          <button :class="{ active: activeTab === 'tag' }" @click="activeTab = 'tag'">{{ $t("app.tabs.tag") }}</button>
        </div>
        <div class="sidebar-content">
          <!-- 按地点 -->
          <LocationList v-if="activeTab === 'location'" @focus-location="onFocusLocation" />
          <!-- 按时间 -->
          <Timeline v-else-if="activeTab === 'timeline'" />
          <!-- 照片列表 -->
          <PhotoList v-else-if="activeTab === 'photos'" @photo-click="openViewer" @locate-photo="onLocatePhoto" />
          <!-- 影集 -->
          <AlbumPanel v-else-if="activeTab === 'album'" @photo-click="openViewer" @locate-photo="onLocatePhoto" />
          <!-- 游记 -->
          <TripList v-else-if="activeTab === 'trip'" @photo-click="openViewer" @locate-photo="onLocatePhoto" />
          <!-- 标签 -->
          <TagPanel v-else-if="activeTab === 'tag'" />
        </div>
        <div class="sidebar-resizer" @mousedown="startResize" />
      </aside>

      <!-- 地图区域 -->
      <main class="map-container">
        <MapView ref="mapRef" @photo-click="openViewer" @map-click="onMapClick" />
      </main>
    </div>

    <!-- 底部状态栏 -->
    <footer class="statusbar">
      <span v-html="$t('app.totalPhotosHtml', { n: store.stats.total })"></span>
      <span v-html="$t('app.locatedHtml', { n: store.stats.located })"></span>
      <span v-html="$t('app.unlocatedHtml', { n: store.stats.unlocated })"></span>
      <span v-if="store.apiConfigured" class="tag" style="background: var(--success-soft); color: var(--success)">{{ $t("app.apiConfigured") }}</span>
      <span v-else class="tag" style="background: var(--warning-soft); color: var(--warning)">{{ $t("app.apiNotConfigured") }}</span>
    </footer>

    <!-- 标注位置弹窗 -->
    <LocationDialog v-if="showLocationDialog" @close="showLocationDialog = false" @done="onLocationDone" />
    <!-- 图片预览弹窗 -->
    <PhotoViewer v-if="viewerPhoto" :photo="viewerPhoto" @close="closeViewer" />
    <!-- 设置弹窗 -->
    <SettingsDialog v-if="showSettings" @close="showSettings = false" />

    <!-- Toast 提示 -->
    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { usePhotoStore } from "./stores/photoStore";
import { debugLog, clearDebugLog, onImportProgress, onGeocodeProgress, cancelLongTask } from "./utils/tauri";
import MapView from "./components/MapView.vue";
import PhotoList from "./components/PhotoList.vue";
import FilterBar from "./components/FilterBar.vue";
import PhotoViewer from "./components/PhotoViewer.vue";
import LocationDialog from "./components/LocationDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import LocationList from "./components/LocationList.vue";
import Timeline from "./components/Timeline.vue";
import AlbumPanel from "./components/AlbumPanel.vue";
import TripList from "./components/TripList.vue";
import TagPanel from "./components/TagPanel.vue";
import ThemeSwitcher from "./components/ThemeSwitcher.vue";
import { loadTheme, setTheme } from "./utils/theme";

const store = usePhotoStore();
const { t } = useI18n();
const showFilterBar = ref(false);
const showSettings = ref(false);
const showLocationDialog = ref(false);
const showThemePopover = ref(false);
const viewerPhoto = ref(null);
const activeTab = ref("location");
const sidebarWidth = ref(280);
const toast = ref(null);
const mapRef = ref(null);
const currentTheme = ref("fresh");

let toastTimer = null;
function showToast(message, duration = 3000) {
  toast.value = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = null), duration);
}

const importProgressPercent = computed(() => {
  if (!store.importProgress || !store.importProgress.total || store.importProgress.scanning) return 0;
  return Math.min(100, Math.round((store.importProgress.done / store.importProgress.total) * 100));
});

async function onCancelLongTask() {
  try {
    await cancelLongTask();
    showToast(t("app.cancelRequested"));
  } catch (e) {
    console.warn("取消失败:", e);
  }
}
const geocodeProgressPercent = computed(() => {
  if (!store.geocodeProgress || !store.geocodeProgress.total) return 0;
  return Math.min(100, Math.round((store.geocodeProgress.done / store.geocodeProgress.total) * 100));
});

async function triggerImportFolder() {
  const selected = await openDialog({
    title: t("app.selectFolderTitle"),
    directory: true,
    multiple: false,
  });
  if (!selected) return;
  try {
    const result = await store.importPhotos([selected], true);
    showToast(t("app.importResult", { success: result.success, skipped: result.skipped, failed: result.failed }));
  } catch (e) {
    showToast(t("app.importFailed", { error: e }));
  }
}

async function triggerImportFiles() {
  const selected = await openDialog({
    title: t("app.selectPhotosTitle"),
    multiple: true,
    filters: [{
      name: t("app.imageFiles"),
      extensions: ["jpg", "jpeg", "png", "tiff", "tif", "webp"],
    }],
  });
  if (!selected || selected.length === 0) return;
  try {
    const result = await store.importPhotos(selected, false);
    showToast(t("app.importResult", { success: result.success, skipped: result.skipped, failed: result.failed }));
  } catch (e) {
    showToast(t("app.importFailed", { error: e }));
  }
}

async function runBatchGeocode() {
  try {
    const updated = await store.runBatchGeocode();
    showToast(t("app.geocodeDone", { n: updated }));
  } catch (e) {
    showToast(t("app.geocodeFailed", { error: e }));
  }
}

function onFocusLocation(payload) {
  const { lat, lng, zoom } = payload || {};
  if (lat != null && lng != null && mapRef.value) {
    mapRef.value.zoomTo(lat, lng, zoom || 12);
  }
}

function openViewer(photo) {
  viewerPhoto.value = photo;
}
function closeViewer() {
  viewerPhoto.value = null;
}

function onLocatePhoto(photo) {
  if (photo.latitude != null && photo.longitude != null && mapRef.value) {
    mapRef.value.zoomToPhoto(photo);
  }
}

function onMapClick(lat, lng) {
  if (store.selectedPhotoIds.length > 0) {
    store.updateLocation(store.selectedPhotoIds, lat, lng);
    showToast(t("app.locationMarked", { n: store.selectedPhotoIds.length }));
  }
}

function onLocationDone() {
  showLocationDialog.value = false;
  store.clearSelection();
  showToast(t("app.locationUpdated"));
}

async function onSelectTheme(id) {
  currentTheme.value = await setTheme(id);
  showThemePopover.value = false;
  showToast(t("app.themeChanged", { name: t("theme." + id + ".name") }));
}

function startResize(e) {
  e.preventDefault();
  const startX = e.clientX;
  const startWidth = sidebarWidth.value;
  const onMove = (ev) => {
    sidebarWidth.value = Math.max(200, Math.min(500, startWidth + ev.clientX - startX));
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

let unlisteners = [];
function onDocClick(e) {
  if (showThemePopover.value && !e.target.closest(".theme-popover-wrap")) {
    showThemePopover.value = false;
  }
}
onMounted(async () => {
  clearDebugLog();
  debugLog("[App] mounted");
  unlisteners.push(await onImportProgress((done, total) => {
    if (done < 0) {
      // 扫描阶段：done=-1，total 为已发现照片数
      store.importProgress = { done: 0, total, scanning: true };
    } else {
      store.importProgress = { done, total, scanning: false };
    }
  }));
  unlisteners.push(await onGeocodeProgress((done, total) => {
    store.geocodeProgress = { done, total };
  }));
  document.addEventListener("click", onDocClick);
  await store.init();
  currentTheme.value = await loadTheme();
});
onUnmounted(() => {
  document.removeEventListener("click", onDocClick);
  unlisteners.forEach((fn) => {
    try { fn(); } catch (e) { /* ignore */ }
  });
});
</script>



<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

/* 顶部工具栏 */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  background: var(--bg2);
  border-bottom: 1px solid var(--rule);
  gap: 1rem;
}
.toolbar-left .app-logo {
  font-weight: 700;
  font-size: 1.1rem;
  background: linear-gradient(135deg, var(--accent), var(--accent2));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.toolbar-center,
.toolbar-right {
  display: flex;
  gap: 0.5rem;
}

/* 一键换肤弹层 */
.theme-popover-wrap {
  position: relative;
}
.theme-popover {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 2100;
  background: var(--bg2);
  border: 1px solid var(--rule);
  border-radius: 12px;
  padding: 0.75rem;
  box-shadow: var(--shadow-lg);
  min-width: 210px;
}
.theme-popover-title {
  font-size: 0.72rem;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}

/* 进度条 */
.progress-container {
  padding: 0.5rem 1rem;
  background: var(--accent-light);
  border-bottom: 1px solid var(--rule);
}
.progress-container span {
  font-size: 0.8rem;
  color: var(--accent);
  display: block;
  margin-bottom: 0.3rem;
}

/* 主体 */
.main-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* 侧边栏 */
.sidebar {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg2);
  border-right: 1px solid var(--rule);
  position: relative;
}
.sidebar-tabs {
  display: flex;
  border-bottom: 1px solid var(--rule);
}
.sidebar-tabs button {
  flex: 1;
  padding: 0.6rem;
  font-size: 0.8rem;
  background: transparent;
  color: var(--muted);
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
}
.sidebar-tabs button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
  font-weight: 600;
}
.sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

/* 侧边栏拖拽调整宽度 */
.sidebar-resizer {
  width: 4px;
  cursor: col-resize;
  position: absolute;
  right: -2px;
  top: 0;
  bottom: 0;
}
.sidebar-resizer:hover {
  background: var(--accent);
}

/* 地图 */
.map-container {
  flex: 1;
  position: relative;
  overflow: hidden;
}

/* 底部状态栏 */
.statusbar {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  padding: 0.4rem 1rem;
  background: var(--bg2);
  border-top: 1px solid var(--rule);
  font-size: 0.78rem;
  color: var(--muted);
}
.statusbar strong {
  color: var(--ink);
}
</style>
