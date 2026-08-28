<template>
  <div class="album-panel">
    <div class="album-header">
      <span class="album-title">{{ $t("album.title") }}</span>
      <div class="album-filter">
        <button class="filter-btn" :class="{ active: groupBy === 'location' }" @click="groupBy = 'location'">
          {{ $t("album.byLocation") }}
        </button>
        <button class="filter-btn" :class="{ active: groupBy === 'time' }" @click="groupBy = 'time'">
          {{ $t("album.byTime") }}
        </button>
      </div>
    </div>

    <div class="album-content">
      <div v-if="albums.length === 0" class="empty-state">
        <div class="empty-icon">📖</div>
        <div class="empty-text">{{ $t("album.noAlbums") }}</div>
        <div class="empty-hint">{{ $t("album.noAlbumsHint") }}</div>
      </div>

      <div v-for="album in pagedAlbums" :key="album.id" class="album-list-item" @click="onOpenAlbum(album)">
        <div class="album-list-thumb">
          <img v-if="album.cover" :src="album.cover" alt="" />
          <div v-else class="album-list-placeholder">
            <span>{{ album.icon }}</span>
          </div>
        </div>
        <div class="album-list-info">
          <div class="album-list-name">{{ album.name }}</div>
          <div class="album-list-meta">
            <span class="meta-count">{{ $t("album.photosCount", { n: album.photos.length }) }}</span>
            <span v-if="album.dateRange" class="meta-date">{{ album.dateRange }}</span>
            <span v-if="album.location" class="meta-location">📍 {{ album.location }}</span>
          </div>
        </div>
        <div class="album-list-arrow">›</div>
      </div>

      <!-- 影集列表分页 -->
      <div v-if="totalAlbumListPages > 1" class="album-list-pagination">
        <button class="page-btn" @click="prevAlbumListPage" :disabled="albumListPage === 1">
          {{ $t("album.prevPage") }}
        </button>
        <span class="page-info">
          {{ albumListPage }} / {{ totalAlbumListPages }}
        </span>
        <button class="page-btn" @click="nextAlbumListPage" :disabled="albumListPage === totalAlbumListPages">
          {{ $t("album.nextPage") }}
        </button>
      </div>
    </div>

    <!-- 影集详情 -->
    <div v-if="currentAlbum" class="album-detail">
      <div class="detail-header">
        <button class="back-btn" @click="currentAlbum = null">{{ $t("album.back") }}</button>
        <div class="detail-title">{{ currentAlbum.name }}</div>
        <div class="detail-count">{{ $t("album.photosCount", { n: currentAlbum.photos.length }) }}</div>
      </div>

      <div class="detail-actions">
        <button class="export-btn" @click="onExportImage" :disabled="!!exporting">
          <span>{{ exporting === 'image' ? $t('album.generating') : $t('album.exportImage') }}</span>
        </button>
        <button class="export-btn" @click="onExportVideo" :disabled="!!exporting">
          <span>{{ exporting === 'video' ? $t('album.recording') : $t('album.exportVideo') }}</span>
        </button>
      </div>

      <div v-if="exportStatus" class="export-status-bar">
        <div class="status-text">{{ exportStatus }}</div>
      </div>

      <div class="detail-photos">
        <div v-for="photo in pagedAlbumPhotos" :key="photo.id" class="detail-photo-item"
          @click="onPhotoClick(photo)">
          <div class="photo-item-thumb">
            <img v-if="thumbMap.get(photo.id)" :src="thumbMap.get(photo.id)" alt="" />
            <div v-else class="thumb-placeholder">🖼️</div>
          </div>
          <div class="photo-item-info">
            <div class="item-name">{{ photo.file_name }}</div>
            <div class="item-meta">
              <span v-if="photo.taken_time" class="item-date">{{ formatAlbumPhotoTime(photo) }}</span>
              <span v-if="photo.city" class="item-city">📍 {{ photo.city }}</span>
            </div>
          </div>
          <div class="photo-item-arrow">›</div>
        </div>
      </div>

      <!-- 分页 -->
      <div v-if="totalAlbumPages > 1" class="album-pagination">
        <button class="page-btn" @click="prevAlbumPage" :disabled="albumPage === 1">
          {{ $t("album.prevPage") }}
        </button>
        <span class="page-info">
          {{ albumPage }} / {{ totalAlbumPages }}
        </span>
        <button class="page-btn" @click="nextAlbumPage" :disabled="albumPage === totalAlbumPages">
          {{ $t("album.nextPage") }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { getImageBase64, exportAlbumImage, exportAlbumVideo, onExportProgress } from "../utils/tauri";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";

const store = usePhotoStore();
const { t } = useI18n();
const emit = defineEmits(["photo-click"]);

const groupBy = ref("location");
const currentAlbum = ref(null);
const thumbMap = ref(new Map());
const exporting = ref(false);
const exportStatus = ref("");

// 分页
const albumPage = ref(1);
const ALBUM_PAGE_SIZE = 8;

// 影集列表分页
const albumListPage = ref(1);
const ALBUM_LIST_PAGE_SIZE = 10;

const totalAlbumListPages = computed(() => {
  return Math.ceil(albums.value.length / ALBUM_LIST_PAGE_SIZE);
});

const pagedAlbums = computed(() => {
  const start = (albumListPage.value - 1) * ALBUM_LIST_PAGE_SIZE;
  const end = start + ALBUM_LIST_PAGE_SIZE;
  return albums.value.slice(start, end);
});

function prevAlbumListPage() {
  if (albumListPage.value > 1) {
    albumListPage.value--;
  }
}

function nextAlbumListPage() {
  if (albumListPage.value < totalAlbumListPages.value) {
    albumListPage.value++;
  }
}

const totalAlbumPages = computed(() => {
  if (!currentAlbum.value) return 0;
  return Math.ceil(currentAlbum.value.photos.length / ALBUM_PAGE_SIZE);
});

const pagedAlbumPhotos = computed(() => {
  if (!currentAlbum.value) return [];
  const start = (albumPage.value - 1) * ALBUM_PAGE_SIZE;
  const end = start + ALBUM_PAGE_SIZE;
  return currentAlbum.value.photos.slice(start, end);
});

function prevAlbumPage() {
  if (albumPage.value > 1) {
    albumPage.value--;
    loadAlbumPageThumbs();
  }
}

function nextAlbumPage() {
  if (albumPage.value < totalAlbumPages.value) {
    albumPage.value++;
    loadAlbumPageThumbs();
  }
}

async function loadAlbumPageThumbs() {
  const photos = pagedAlbumPhotos.value;
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

function formatAlbumPhotoTime(photo) {
  if (!photo.taken_time) return "";
  const d = new Date(photo.taken_time);
  if (isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

let unlistenProgress = null;
let thumbLoadCancelled = false;
let componentAlive = false;

onMounted(async () => {
  componentAlive = true;
  const fn = await onExportProgress((done, total) => {
    if (done < 0) {
      exportStatus.value = t("album.encodingVideo");
    } else if (total > 0) {
      exportStatus.value = t("album.processingProgress", { done, total });
    }
  });
  if (componentAlive) {
    unlistenProgress = fn;
  } else {
    fn();
  }
});

onUnmounted(() => {
  componentAlive = false;
  thumbLoadCancelled = true;
  if (unlistenProgress) unlistenProgress();
});

watch(() => store.photos, async () => {
  if (thumbLoadCancelled) return;
  for (const p of store.photos) {
    if (thumbLoadCancelled) return;
    if (p.thumbnail_path && !thumbMap.value.has(p.id)) {
      try {
        const b64 = await getImageBase64(p.thumbnail_path);
        if (b64 && !thumbLoadCancelled) thumbMap.value.set(p.id, b64);
      } catch (e) {}
    }
  }
}, { immediate: true, deep: false });

const albums = computed(() => {
  const photos = store.photos;
  if (photos.length === 0) return [];

  const groups = new Map();

  if (groupBy.value === "location") {
    for (const p of photos) {
      const key = p.city || (p.latitude != null ? `坐标 ${p.latitude.toFixed(1)}, ${p.longitude.toFixed(1)}` : t("album.uncategorized"));
      if (!groups.has(key)) {
        const name = p.city || (p.latitude != null ? t("album.coordKey", { lat: p.latitude.toFixed(1), lng: p.longitude.toFixed(1) }) : t("album.uncategorized"));
        groups.set(key, { id: "loc_" + key, name, location: p.province || "", icon: "📍", photos: [] });
      }
      groups.get(key).photos.push(p);
    }
  } else if (groupBy.value === "time") {
    for (const p of photos) {
      if (!p.taken_time) continue;
      const d = new Date(p.taken_time * 1000);
      const key = `${d.getFullYear()}-${d.getMonth() + 1}`;
      if (!groups.has(key)) {
        groups.set(key, { id: "time_" + key, name: t("album.yearMonth", { y: d.getFullYear(), m: d.getMonth() + 1 }), icon: "📅", photos: [] });
      }
      groups.get(key).photos.push(p);
    }
    const noTime = photos.filter(p => !p.taken_time);
    if (noTime.length > 0) {
      groups.set("unknown", { id: "time_unknown", name: t("album.unknownTime"), icon: "❓", photos: noTime });
    }
  }

  const result = [];
  for (const album of groups.values()) {
    if (album.photos.length < 1) continue;
    const coverPhoto = album.photos.find(p => p.thumbnail_path);
    if (coverPhoto && thumbMap.value.has(coverPhoto.id)) {
      album.cover = thumbMap.value.get(coverPhoto.id);
    } else {
      album.cover = null;
    }
    const times = album.photos.filter(p => p.taken_time).map(p => p.taken_time);
    if (times.length > 0) {
      const minTime = new Date(Math.min(...times) * 1000);
      const maxTime = new Date(Math.max(...times) * 1000);
      const fmt = (d) => `${d.getFullYear()}.${d.getMonth() + 1}.${d.getDate()}`;
      album.dateRange = times.length > 1 ? `${fmt(minTime)} - ${fmt(maxTime)}` : fmt(minTime);
    } else {
      album.dateRange = "";
    }
    album.photos.sort((a, b) => (b.taken_time || 0) - (a.taken_time || 0));
    result.push(album);
  }
  result.sort((a, b) => b.photos.length - a.photos.length);
  return result;
});

function onOpenAlbum(album) {
  currentAlbum.value = album;
  albumPage.value = 1;
  loadAlbumPageThumbs();
}

function onPhotoClick(photo) {
  emit("photo-click", photo);
}

async function onExportImage() {
  if (!currentAlbum.value || exporting.value) return;
  const album = currentAlbum.value;
  exporting.value = "image";
  exportStatus.value = t("album.preparing");

  try {
    const filePath = await saveDialog({
      title: t("album.saveAlbumImageTitle"),
      defaultPath: t("album.defaultImageName", { name: album.name }),
      filters: [{ name: t("album.pngImages"), extensions: ["png"] }],
    });
    if (!filePath) return;

    exportStatus.value = t("album.generatingCollage");
    const photoPaths = album.photos.map(p => p.file_path).filter(Boolean);
    const result = await exportAlbumImage(
      album.name,
      album.dateRange || "",
      album.location || "",
      photoPaths,
      filePath,
    );
    exportStatus.value = t("album.exportSuccess");
    setTimeout(() => { exportStatus.value = ""; }, 3000);
  } catch (e) {
    exportStatus.value = t("album.exportFailed", { error: e });
    console.error(e);
  } finally {
    setTimeout(() => { exporting.value = false; }, 500);
  }
}

async function onExportVideo() {
  if (!currentAlbum.value || exporting.value) return;
  const album = currentAlbum.value;
  exporting.value = "video";
  exportStatus.value = t("album.preparing");

  try {
    const filePath = await saveDialog({
      title: t("album.saveAlbumVideoTitle"),
      defaultPath: t("album.defaultVideoName", { name: album.name }),
      filters: [
        { name: t("album.mp4Video"), extensions: ["mp4"] },
        { name: t("album.webmVideo"), extensions: ["webm"] },
      ],
    });
    if (!filePath) return;

    exportStatus.value = t("album.generatingFrames");
    const photoPaths = album.photos.map(p => p.file_path).filter(Boolean);
    const result = await exportAlbumVideo(
      album.name,
      album.dateRange || "",
      photoPaths,
      filePath,
    );
    const ext = result.split('.').pop();
    exportStatus.value = t("album.exportSuccessExt", { ext });
    setTimeout(() => { exportStatus.value = ""; }, 3000);
  } catch (e) {
    exportStatus.value = t("album.exportFailed", { error: e });
    console.error(e);
  } finally {
    setTimeout(() => { exporting.value = false; }, 500);
  }
}
</script>

<style scoped>
.album-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
}

.album-header {
  padding: 12px 14px 8px;
}
.album-title {
  display: block;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
  margin-bottom: 8px;
}
.album-filter {
  display: flex;
  gap: 4px;
  background: var(--bg-card);
  padding: 3px;
  border-radius: var(--radius-sm);
}
.filter-btn {
  flex: 1;
  padding: 5px 0;
  font-size: 0.72rem;
  border-radius: 5px;
  color: var(--text-muted);
  transition: all 0.15s ease;
}
.filter-btn:hover {
  color: var(--text-secondary);
}
.filter-btn.active {
  background: var(--bg-panel-solid);
  color: var(--accent);
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.album-content {
  flex: 1;
  overflow-y: auto;
  padding: 4px 14px 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.album-list-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--radius);
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
  background: var(--bg-panel);
}
.album-list-item:hover {
  background: var(--bg-hover);
  border-color: var(--border);
}
.album-list-thumb {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
}
.album-list-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.album-list-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  color: rgba(255, 255, 255, 0.8);
}
.album-list-info {
  flex: 1;
  min-width: 0;
}
.album-list-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}
.album-list-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 0.7rem;
  color: var(--text-muted);
}
.album-list-meta .meta-count {
  color: var(--accent);
  font-weight: 500;
}
.album-list-arrow {
  color: var(--text-muted);
  font-size: 1.2rem;
  flex-shrink: 0;
}

.album-list-pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 12px 0;
  margin-top: auto;
  flex-shrink: 0;
}
.album-list-pagination .page-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.72rem;
  color: var(--text-secondary);
  background: var(--bg-hover);
  transition: all 0.15s ease;
}
.album-list-pagination .page-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.album-list-pagination .page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.album-list-pagination .page-info {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-muted);
}
.empty-icon { font-size: 2.5rem; margin-bottom: 12px; opacity: 0.4; }
.empty-text { font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 4px; }
.empty-hint { font-size: 0.75rem; color: var(--text-muted); text-align: center; }

/* 影集详情 */
.album-detail {
  position: absolute;
  inset: 0;
  background: var(--bg-panel-solid);
  z-index: 10;
  display: flex;
  flex-direction: column;
}
.detail-header {
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 10px;
}
.back-btn {
  font-size: 0.8rem;
  color: var(--accent);
  padding: 4px 8px;
  border-radius: var(--radius-sm);
}
.back-btn:hover {
  background: rgba(14, 165, 233, 0.08);
}
.detail-title {
  flex: 1;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}
.detail-count {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.detail-actions {
  display: flex;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}
.export-btn {
  flex: 1;
  padding: 8px 0;
  font-size: 0.78rem;
  font-weight: 600;
  border-radius: var(--radius-sm);
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  color: #fff;
  transition: opacity 0.15s ease;
}
.export-btn:hover:not(:disabled) {
  opacity: 0.9;
}
.export-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-status-bar {
  padding: 8px 14px;
  background: rgba(14, 165, 233, 0.06);
  border-bottom: 1px solid var(--border);
}
.status-text {
  font-size: 0.75rem;
  color: var(--accent);
  font-weight: 500;
}

.detail-photos {
  flex: 1;
  overflow-y: auto;
  padding: 8px 16px;
}

.detail-photo-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
}
.detail-photo-item:hover {
  background: var(--bg-hover);
  border-color: var(--border-glow);
}
.photo-item-thumb {
  width: 48px;
  height: 48px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-card);
}
.photo-item-thumb img {
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
  color: var(--text-muted);
  background: var(--bg-card);
}
.photo-item-info {
  flex: 1;
  min-width: 0;
}
.photo-item-info .item-name {
  font-size: 0.78rem;
  color: var(--text-primary);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.photo-item-info .item-meta {
  margin-top: 2px;
  display: flex;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--text-muted);
}
.photo-item-info .item-date,
.photo-item-info .item-city {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.photo-item-arrow {
  color: var(--text-muted);
  font-size: 1rem;
  flex-shrink: 0;
}

.album-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}
.album-pagination .page-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.72rem;
  color: var(--text-secondary);
  background: var(--bg-hover);
  transition: all 0.15s ease;
}
.album-pagination .page-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.album-pagination .page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.album-pagination .page-info {
  font-size: 0.72rem;
  color: var(--text-muted);
}
</style>
