<template>
  <div class="viewer-overlay" @click.self="$emit('close')" tabindex="0" @keydown="onKeydown">
    <div class="viewer-container">
      <button class="viewer-close" @click="$emit('close')">✕</button>

      <!-- 上一张按钮 -->
      <button v-if="hasPrev" class="nav-btn nav-prev" @click="goPrev" title="上一张 (←)">
        ‹
      </button>
      <!-- 下一张按钮 -->
      <button v-if="hasNext" class="nav-btn nav-next" @click="goNext" title="下一张 (→)">
        ›
      </button>

      <div class="viewer-image">
        <img v-if="imageSrc" :src="imageSrc" :alt="currentPhoto.file_name" />
        <div v-else class="img-loading">
          <div class="loading-spinner"></div>
          <span>加载中...</span>
        </div>
      </div>

      <div class="viewer-sidebar">
        <h3 class="viewer-title">{{ currentPhoto.file_name }}</h3>

        <div class="info-section">
          <div class="info-label">拍摄信息</div>
          <div class="info-row">
            <span class="info-key">拍摄时间</span>
            <span class="info-val">{{ formatDateTime(currentPhoto.taken_time) }}</span>
          </div>
          <div class="info-row">
            <span class="info-key">相机</span>
            <span class="info-val">{{ currentPhoto.camera_model || "—" }}</span>
          </div>
        </div>

        <div class="info-section">
          <div class="info-label">位置信息</div>
          <div v-if="currentPhoto.address" class="info-row highlight">
            <span class="info-key">📍 地址</span>
            <span class="info-val addr highlight-val">{{ currentPhoto.address }}</span>
          </div>
          <div v-if="currentPhoto.city || currentPhoto.province" class="info-row">
            <span class="info-key">🏙️ 城市</span>
            <span class="info-val">
              <span v-if="currentPhoto.province">{{ currentPhoto.province }}</span>
              <span v-if="currentPhoto.province && currentPhoto.city"> · </span>
              <span v-if="currentPhoto.city">{{ currentPhoto.city }}</span>
            </span>
          </div>
          <div v-if="currentPhoto.latitude != null" class="info-row coords-row">
            <span class="info-key">经纬</span>
            <span class="info-val coords-val">{{ currentPhoto.latitude?.toFixed(4) }}, {{ currentPhoto.longitude?.toFixed(4) }}</span>
          </div>
          <div v-if="currentPhoto.latitude == null && !currentPhoto.address" class="info-row empty">
            <span class="info-val empty-val">暂无位置信息</span>
          </div>
        </div>

        <div class="info-section">
          <div class="info-label">文件信息</div>
          <div class="info-row">
            <span class="info-key">大小</span>
            <span class="info-val">{{ formatSize(currentPhoto.file_size) }}</span>
          </div>
          <div class="info-row">
            <span class="info-key">路径</span>
            <span class="info-val path" :title="currentPhoto.file_path">{{ currentPhoto.file_path }}</span>
          </div>
        </div>

        <!-- 照片计数 -->
        <div v-if="photoList.length > 1" class="photo-counter">
          {{ currentIndex + 1 }} / {{ photoList.length }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { getImageBase64 } from "../utils/tauri";

const props = defineProps({
  photo: { type: Object, required: true },
  photoList: { type: Array, default: () => [] },
});
const emit = defineEmits(["close"]);

const currentIndex = ref(0);
const imageSrc = ref(null);

const currentPhoto = computed(() => {
  if (props.photoList.length > 0 && currentIndex.value < props.photoList.length) {
    return props.photoList[currentIndex.value];
  }
  return props.photo;
});

const hasPrev = computed(() => props.photoList.length > 1 && currentIndex.value > 0);
const hasNext = computed(() => props.photoList.length > 1 && currentIndex.value < props.photoList.length - 1);

function formatDateTime(ts) {
  if (!ts) return "—";
  const d = new Date(ts);
  if (isNaN(d.getTime())) return "—";
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;
}

function formatSize(bytes) {
  if (!bytes) return "—";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
}

async function loadImage() {
  imageSrc.value = null;
  const photo = currentPhoto.value;
  const src = await getImageBase64(photo.file_path);
  if (src) {
    imageSrc.value = src;
  } else if (photo.thumbnail_path) {
    imageSrc.value = await getImageBase64(photo.thumbnail_path);
  }
}

function goPrev() {
  if (hasPrev.value) {
    currentIndex.value--;
    loadImage();
  }
}

function goNext() {
  if (hasNext.value) {
    currentIndex.value++;
    loadImage();
  }
}

function onKeydown(e) {
  if (e.key === "ArrowLeft") {
    e.preventDefault();
    goPrev();
  } else if (e.key === "ArrowRight") {
    e.preventDefault();
    goNext();
  } else if (e.key === "Escape") {
    e.preventDefault();
    emit("close");
  }
}

// 全局键盘监听
function onGlobalKeydown(e) {
  if (e.key === "ArrowLeft") {
    e.preventDefault();
    goPrev();
  } else if (e.key === "ArrowRight") {
    e.preventDefault();
    goNext();
  } else if (e.key === "Escape") {
    e.preventDefault();
    emit("close");
  }
}

onMounted(async () => {
  // 如果有照片列表，找到当前照片的索引
  if (props.photoList.length > 0) {
    const idx = props.photoList.findIndex(p => p.id === props.photo.id);
    currentIndex.value = idx >= 0 ? idx : 0;
  }
  await loadImage();
  // 添加全局键盘监听
  window.addEventListener("keydown", onGlobalKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

// 当 photo 变化时（从外部切换），重新查找索引
watch(() => props.photo, (newPhoto) => {
  if (props.photoList.length > 0 && newPhoto) {
    const idx = props.photoList.findIndex(p => p.id === newPhoto.id);
    if (idx >= 0 && idx !== currentIndex.value) {
      currentIndex.value = idx;
      loadImage();
    }
  }
});
</script>

<style scoped>
.viewer-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  animation: fadeIn 0.2s ease;
}
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.viewer-container {
  display: flex;
  max-width: 90vw;
  max-height: 85vh;
  background: var(--bg-panel-solid);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px var(--accent-glow);
  position: relative;
}

.viewer-close {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  backdrop-filter: blur(4px);
}
.viewer-close:hover {
  background: rgba(239, 68, 68, 0.6);
}

/* 导航按钮 */
.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  font-size: 1.8rem;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  backdrop-filter: blur(4px);
  transition: all 0.15s ease;
  line-height: 1;
}
.nav-btn:hover {
  background: rgba(99, 102, 241, 0.7);
  transform: translateY(-50%) scale(1.1);
}
.nav-prev {
  left: 12px;
}
.nav-next {
  right: 312px;
}

.viewer-image {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #000;
  min-width: 400px;
  min-height: 300px;
}
.viewer-image img {
  max-width: 100%;
  max-height: 85vh;
  object-fit: contain;
}

.img-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
  font-size: 0.85rem;
}
.loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.viewer-sidebar {
  width: 300px;
  padding: 20px;
  border-left: 1px solid var(--border);
  overflow-y: auto;
}

.viewer-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 16px;
  word-break: break-all;
}

.info-section {
  margin-bottom: 16px;
}
.info-label {
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--accent);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}

.info-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 5px 0;
  font-size: 0.78rem;
  flex-wrap: wrap;
}
.info-row.highlight {
  padding: 10px 12px;
  margin-bottom: 6px;
  background: rgba(122, 155, 132, 0.1);
  border-radius: 8px;
  border: 1px solid rgba(122, 155, 132, 0.2);
}
.info-row.highlight .info-key {
  color: var(--accent);
  font-weight: 600;
}
.info-row.highlight .highlight-val {
  color: var(--text-primary);
  font-weight: 500;
  line-height: 1.5;
}
.info-row.coords-row {
  padding: 4px 0;
  opacity: 0.7;
}
.info-row.coords-row .coords-val {
  font-size: 0.7rem;
  font-family: monospace;
}
.info-row.empty {
  align-items: center;
  padding: 8px 0;
}
.info-row.empty .empty-val {
  color: var(--text-muted);
  font-size: 0.75rem;
}
.info-key {
  color: var(--text-muted);
  font-size: 0.72rem;
  flex-shrink: 0;
  min-width: 50px;
}
.info-val {
  color: var(--text-secondary);
  text-align: left;
  word-break: break-all;
  flex: 1;
}
.info-val.addr {
  line-height: 1.4;
}
.info-val.path {
  font-size: 0.7rem;
  font-family: monospace;
  line-height: 1.3;
}

/* 照片计数 */
.photo-counter {
  margin-top: 16px;
  text-align: center;
  font-size: 0.75rem;
  color: var(--text-muted);
  padding: 6px;
  background: var(--bg-card);
  border-radius: 6px;
}
</style>
