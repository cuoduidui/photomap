<template>
  <div class="photo-list">
    <!-- 筛选 -->
    <div class="filter-bar">
      <div class="filter-tabs">
        <button class="filter-tab" :class="{ active: filter === 'all' }" @click="filter = 'all'">
          全部 <span class="count">{{ store.photos.length }}</span>
        </button>
        <button class="filter-tab" :class="{ active: filter === 'located' }" @click="filter = 'located'">
          已定位 <span class="count">{{ store.stats.located }}</span>
        </button>
        <button class="filter-tab" :class="{ active: filter === 'unlocated' }" @click="filter = 'unlocated'">
          未定位 <span class="count">{{ store.stats.unlocated }}</span>
        </button>
      </div>
      <div v-if="store.filter.city || store.filter.province" class="filter-active">
        <span class="filter-active-text">
          {{ store.filter.city || store.filter.province }}
        </span>
        <button class="filter-clear" @click="clearLocationFilter">✕</button>
      </div>
    </div>

    <!-- 批量操作栏 -->
    <div v-if="store.selectedPhotoIds.length > 0" class="batch-bar">
      <div class="batch-info">
        <span class="batch-count">已选 {{ store.selectedPhotoIds.length }} 张</span>
        <button class="batch-clear" @click="store.clearSelection()">取消选择</button>
      </div>
      <div class="batch-actions">
        <button class="batch-btn locate-btn" @click="openLocationEditor" title="修改位置">
          📍 修改位置
        </button>
        <button class="batch-btn delete-btn" @click="batchDelete" title="删除">
          🗑️ 删除
        </button>
      </div>
    </div>

    <!-- 照片网格 - 虚拟滚动 -->
    <div ref="gridContainerRef" class="photo-grid-container" @scroll="onScroll">
      <div class="photo-grid-spacer" :style="{ height: totalHeight + 'px' }">
        <div class="photo-grid" :style="{ transform: `translateY(${startOffset}px)`, gridTemplateColumns: `repeat(${columns}, 1fr)` }">
          <div v-for="p in visiblePhotos" :key="p.id" class="photo-card"
            :class="{ selected: store.selectedPhotoIds.includes(p.id) }"
            @click="onPhotoClick(p)"
            @dblclick="emit('photo-click', p)">
            <div class="photo-check" @click.stop="store.togglePhotoSelection(p.id)">
              <span v-if="store.selectedPhotoIds.includes(p.id)">✓</span>
            </div>
            <div class="photo-actions">
              <button v-if="p.latitude != null" class="action-btn" @click.stop="onLocate(p)" title="在地图中定位">
                📍
              </button>
              <button class="action-btn delete" @click.stop="onDelete(p)" title="删除">
                🗑️
              </button>
            </div>
            <div class="photo-image">
              <img v-if="thumbMap.get(p.id)" :src="thumbMap.get(p.id)" :alt="p.file_name" />
              <div v-else-if="thumbFailed.has(p.id)" class="img-fallback">
                <span class="fallback-icon">🖼️</span>
              </div>
              <div v-else class="img-skeleton">
                <div class="skeleton-shimmer"></div>
              </div>
              <div class="photo-overlay">
                <div class="overlay-date">{{ formatPhotoDate(p) }}</div>
              </div>
            </div>
            <div class="photo-info">
              <div class="photo-name" :title="p.file_name">{{ p.file_name }}</div>
              <div class="photo-meta">
                <span v-if="p.latitude != null" class="meta-tag located" :title="p.address || '已定位'">
                  📍 {{ p.city || '已定位' }}
                </span>
                <span v-else class="meta-tag unlocated">未定位</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="filteredPhotos.length === 0" class="empty-state">
        <div class="empty-icon">📷</div>
        <div class="empty-text">暂无照片</div>
        <div class="empty-hint">点击"导入文件夹"或"选择图片"开始</div>
      </div>
    </div>

    <!-- 位置编辑弹窗 -->
    <div v-if="showLocationEditor" class="location-editor-overlay" @click="showLocationEditor = false">
      <div class="location-editor-modal" @click.stop>
        <div class="editor-header">
          <span class="editor-title">修改位置信息</span>
          <button class="editor-close" @click="showLocationEditor = false">✕</button>
        </div>
        <div class="editor-body">
          <div class="editor-desc">
            将为选中的 {{ store.selectedPhotoIds.length }} 张照片设置位置
          </div>
          <div class="editor-field">
            <label class="field-label">搜索位置</label>
            <div class="search-row">
              <input type="text" v-model="searchKeyword" class="search-input" 
                placeholder="输入地点名称搜索..." @keyup.enter="searchLocation" />
              <button class="search-btn" @click="searchLocation" :disabled="searching">
                {{ searching ? '搜索中...' : '搜索' }}
              </button>
            </div>
          </div>
          <div v-if="searchResults.length > 0" class="search-results">
            <div v-for="(result, idx) in searchResults" :key="idx" 
              class="search-result-item"
              @click="selectLocation(result)">
              <div class="result-name">{{ result.name }}</div>
              <div class="result-address">{{ result.address }}</div>
            </div>
          </div>
          <div class="editor-field">
            <label class="field-label">或手动输入坐标</label>
            <div class="coord-row">
              <div class="coord-input">
                <span class="coord-label">纬度</span>
                <input type="number" v-model.number="latInput" step="0.000001" placeholder="如: 39.9042" />
              </div>
              <div class="coord-input">
                <span class="coord-label">经度</span>
                <input type="number" v-model.number="lngInput" step="0.000001" placeholder="如: 116.4074" />
              </div>
            </div>
          </div>
          <div class="editor-field">
            <label class="field-label">地址描述（可选）</label>
            <input type="text" v-model="addressInput" class="address-input" 
              placeholder="如: 北京市东城区天安门广场" />
          </div>
        </div>
        <div class="editor-footer">
          <button class="cancel-btn" @click="showLocationEditor = false">取消</button>
          <button class="confirm-btn" @click="confirmLocation" 
            :disabled="!canConfirm || saving">
            {{ saving ? '保存中...' : '确认修改' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, nextTick, onUnmounted } from "vue";
import { usePhotoStore } from "../stores/photoStore";
import { getImageBase64, searchAddress } from "../utils/tauri";

const emit = defineEmits(["locate-photo", "photo-click"]);
const store = usePhotoStore();
const filter = ref("all");
const thumbMap = ref(new Map());
const thumbFailed = ref(new Set());

const showLocationEditor = ref(false);
const searchKeyword = ref("");
const searchResults = ref([]);
const searching = ref(false);
const latInput = ref(null);
const lngInput = ref(null);
const addressInput = ref("");
const saving = ref(false);

// 虚拟滚动（动态计算列数与行高）
const gridContainerRef = ref(null);
const scrollTop = ref(0);
const containerWidth = ref(300);
const containerHeight = ref(600);
const COL_GAP = 8;
const GRID_PADDING = 8;
const INFO_HEIGHT = 42; // 信息区高度：padding(12) + name(14) + meta(12) + border(1) ≈ 39，留少量余量
const BUFFER_ROWS = 4; // 上下各缓冲几行

const filteredPhotos = computed(() => {
  // 基于全局筛选（日期/地点/坐标/人物/关键词），与地图保持一致
  let result = store.filteredPhotos;
  if (filter.value === "located") {
    result = result.filter(p => p.latitude != null && p.longitude != null);
  } else if (filter.value === "unlocated") {
    result = result.filter(p => p.latitude == null || p.longitude == null);
  }
  return result;
});

// 动态列数：每列最小宽度 104px
const columns = computed(() => {
  const usable = containerWidth.value - GRID_PADDING * 2;
  const n = Math.floor(usable / 104);
  return Math.max(2, Math.min(6, n));
});

// 动态卡片高度 = 图片(正方形，宽=列宽) + 信息区 + 边框
const colWidth = computed(() => {
  if (columns.value <= 0) return 100;
  const usable = containerWidth.value - GRID_PADDING * 2 - COL_GAP * (columns.value - 1);
  return Math.max(80, Math.floor(usable / columns.value));
});

const cardHeight = computed(() => colWidth.value + INFO_HEIGHT);
const rowStride = computed(() => cardHeight.value + COL_GAP);

const totalRows = computed(() => Math.ceil(filteredPhotos.value.length / columns.value));
const totalHeight = computed(() => {
  if (totalRows.value === 0) return 0;
  return totalRows.value * cardHeight.value + (totalRows.value - 1) * COL_GAP + GRID_PADDING * 2;
});

const startRow = computed(() => {
  if (rowStride.value <= 0) return 0;
  const row = Math.floor(scrollTop.value / rowStride.value);
  return Math.max(0, row - BUFFER_ROWS);
});

const endRow = computed(() => {
  const visibleRows = Math.ceil(containerHeight.value / rowStride.value);
  const row = startRow.value + visibleRows + BUFFER_ROWS * 2;
  return Math.min(totalRows.value, row);
});

const startOffset = computed(() => {
  return startRow.value * rowStride.value;
});

const visiblePhotos = computed(() => {
  const start = startRow.value * columns.value;
  const end = endRow.value * columns.value;
  return filteredPhotos.value.slice(start, end);
});

const canConfirm = computed(() => {
  return latInput.value != null && lngInput.value != null && 
    !isNaN(latInput.value) && !isNaN(lngInput.value);
});

function onScroll(e) {
  scrollTop.value = e.target.scrollTop;
  // 滚动时延迟加载缩略图
  requestAnimationFrame(() => {
    loadVisibleThumbs();
  });
}

// 缩略图并发加载
const MAX_CACHE_SIZE = 300;
const inFlight = new Set();
let isLoading = false;

function trimCache() {
  if (thumbMap.value.size > MAX_CACHE_SIZE) {
    const keys = Array.from(thumbMap.value.keys());
    const toDelete = keys.slice(0, Math.floor(MAX_CACHE_SIZE / 2));
    for (const k of toDelete) {
      thumbMap.value.delete(k);
    }
  }
}

async function loadVisibleThumbs() {
  if (isLoading) return;

  const visible = visiblePhotos.value;
  const toLoad = visible.filter(p =>
    p.thumbnail_path &&
    !thumbMap.value.has(p.id) &&
    !thumbFailed.value.has(p.id) &&
    !inFlight.has(p.id)
  );

  if (toLoad.length === 0) return;

  isLoading = true;
  try {
    // 每次最多加载 24 张，分批并发 4 张，避免一次性占用过多 IPC
    const batch = toLoad.slice(0, 24);
    for (let i = 0; i < batch.length; i += 4) {
      const group = batch.slice(i, i + 4);
      await Promise.all(group.map(async (photo) => {
        inFlight.add(photo.id);
        try {
          const b64 = await getImageBase64(photo.thumbnail_path);
          if (b64) {
            thumbMap.value.set(photo.id, b64);
            trimCache();
          } else {
            thumbFailed.value.add(photo.id);
          }
        } catch {
          thumbFailed.value.add(photo.id);
        } finally {
          inFlight.delete(photo.id);
        }
      }));
      // 每批完成后触发一次响应式更新
      thumbMap.value = new Map(thumbMap.value);
    }
  } finally {
    isLoading = false;
    // 如果当前可见区域仍有未加载的（例如滚动后新增），继续加载
    const stillPending = visiblePhotos.value.some(p =>
      p.thumbnail_path &&
      !thumbMap.value.has(p.id) &&
      !thumbFailed.value.has(p.id) &&
      !inFlight.has(p.id)
    );
    if (stillPending) {
      requestAnimationFrame(() => loadVisibleThumbs());
    }
  }
}

function onPhotoClick(photo) {
  store.togglePhotoSelection(photo.id);
  if (photo.latitude != null && photo.longitude != null) {
    emit("locate-photo", photo);
  }
}

function onLocate(photo) {
  emit("locate-photo", photo);
}

async function onDelete(photo) {
  if (!confirm(`确定要删除 "${photo.file_name}" 吗？`)) return;
  try {
    await store.deletePhotos([photo.id]);
  } catch (e) {
    console.warn("删除失败:", e);
  }
}

async function batchDelete() {
  const count = store.selectedPhotoIds.length;
  if (!confirm(`确定要删除选中的 ${count} 张照片吗？此操作不可撤销。`)) return;
  try {
    await store.deletePhotos([...store.selectedPhotoIds]);
  } catch (e) {
    console.warn("批量删除失败:", e);
  }
}

function openLocationEditor() {
  latInput.value = null;
  lngInput.value = null;
  addressInput.value = "";
  searchKeyword.value = "";
  searchResults.value = [];
  showLocationEditor.value = true;
}

async function searchLocation() {
  if (!searchKeyword.value.trim()) return;
  searching.value = true;
  searchResults.value = [];
  try {
    const results = await searchAddress(searchKeyword.value);
    searchResults.value = results || [];
  } catch (e) {
    console.warn("搜索失败:", e);
  } finally {
    searching.value = false;
  }
}

function selectLocation(result) {
  latInput.value = result.latitude;
  lngInput.value = result.longitude;
  addressInput.value = result.address || result.name;
}

async function confirmLocation() {
  if (!canConfirm.value) return;
  saving.value = true;
  try {
    await store.updateLocation(
      [...store.selectedPhotoIds],
      latInput.value,
      lngInput.value,
      addressInput.value || null
    );
    showLocationEditor.value = false;
    store.clearSelection();
  } catch (e) {
    console.warn("修改位置失败:", e);
    alert("修改位置失败: " + e);
  } finally {
    saving.value = false;
  }
}

function clearLocationFilter() {
  store.setFilter({ province: null, city: null });
}

function formatPhotoDate(photo) {
  if (!photo.taken_time) return "";
  const d = new Date(photo.taken_time);
  if (isNaN(d.getTime())) return "";
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

// 更新容器尺寸
function updateContainerSize() {
  if (gridContainerRef.value) {
    containerWidth.value = gridContainerRef.value.clientWidth || containerWidth.value;
    containerHeight.value = gridContainerRef.value.clientHeight || containerHeight.value;
  }
}

let resizeObserver = null;
let sizeTimer = null;

// 照片数量变化（如导入/删除）：保持滚动位置，仅刷新缩略图
watch(() => store.photos.length, () => {
  nextTick(() => {
    loadVisibleThumbs();
  });
});

// 筛选条件变化：重置滚动位置并重新加载
watch(
  [filter, () => store.filter],
  () => {
    scrollTop.value = 0;
    if (gridContainerRef.value) {
      gridContainerRef.value.scrollTop = 0;
    }
    nextTick(() => loadVisibleThumbs());
  },
  { deep: true }
);

onMounted(() => {
  updateContainerSize();
  loadVisibleThumbs();
  // 监听容器尺寸变化（面板宽度变化时动态调整列数）
  if (window.ResizeObserver && gridContainerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      updateContainerSize();
      if (sizeTimer) clearTimeout(sizeTimer);
      sizeTimer = setTimeout(() => {
        // 尺寸变化后修正可能越界的滚动位置
        if (gridContainerRef.value) {
          const maxScroll = gridContainerRef.value.scrollHeight - gridContainerRef.value.clientHeight;
          if (gridContainerRef.value.scrollTop > maxScroll) {
            gridContainerRef.value.scrollTop = Math.max(0, maxScroll);
          }
        }
        // 重新计算可见区域并加载缩略图
        nextTick(() => loadVisibleThumbs());
      }, 150);
    });
    resizeObserver.observe(gridContainerRef.value);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
  if (sizeTimer) {
    clearTimeout(sizeTimer);
  }
});
</script>

<style scoped>
.photo-list {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.filter-bar {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}

.filter-tabs {
  display: flex;
  gap: 4px;
}

.filter-tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--text-secondary);
  transition: all 0.2s ease;
}
.filter-tab:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.filter-tab.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.filter-tab .count {
  font-size: 0.65rem;
  opacity: 0.7;
}

.filter-active {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin-top: 8px;
  padding: 4px 10px;
  background: var(--accent2-soft);
  border: 1px solid var(--accent2-soft);
  border-radius: 12px;
  font-size: 0.72rem;
}
.filter-active-text {
  color: var(--accent-2);
  font-weight: 500;
}
.filter-clear {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: rgba(192, 133, 82, 0.15);
  color: var(--accent-2);
  font-size: 0.6rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.filter-clear:hover {
  background: var(--danger-soft);
  color: var(--error);
}

/* 批量操作栏 */
.batch-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: linear-gradient(135deg, var(--accent-soft), var(--accent2-soft));
  border-bottom: 1px solid var(--accent-soft);
  animation: slideDown 0.2s ease;
  flex-shrink: 0;
}
.batch-info {
  display: flex;
  align-items: center;
  gap: 10px;
}
.batch-count {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--accent-2);
}
.batch-clear {
  font-size: 0.7rem;
  color: var(--text-muted);
  text-decoration: underline;
}
.batch-clear:hover {
  color: var(--text-secondary);
}
.batch-actions {
  display: flex;
  gap: 8px;
}
.batch-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border-radius: 8px;
  font-size: 0.72rem;
  font-weight: 500;
  transition: all 0.2s ease;
}
.batch-btn:hover {
  transform: translateY(-1px);
}
.locate-btn {
  background: var(--accent-soft);
  color: var(--accent);
}
.locate-btn:hover {
  background: rgba(122, 155, 132, 0.25);
}
.delete-btn {
  background: var(--danger-soft);
  color: var(--error);
}
.delete-btn:hover {
  background: var(--danger-soft);
}

@keyframes slideDown {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 照片网格容器 - 虚拟滚动 */
.photo-grid-container {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  -webkit-overflow-scrolling: touch;
}

.photo-grid-spacer {
  position: relative;
  width: 100%;
}

/* 照片网格 - 动态列布局 */
.photo-grid {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  padding: 8px;
  display: grid;
  gap: 8px;
}

.photo-card {
  position: relative;
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--bg-card);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: all 0.2s ease;
}
.photo-card:hover {
  border-color: var(--border-glow);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}
.photo-card.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft), 0 0 12px var(--accent-glow);
}

.photo-check {
  position: absolute;
  top: 6px;
  left: 6px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.6);
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.7rem;
  color: #fff;
  z-index: 3;
  transition: all 0.2s ease;
  backdrop-filter: blur(4px);
}
.photo-card.selected .photo-check {
  background: var(--accent);
  border-color: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}

.photo-actions {
  position: absolute;
  top: 6px;
  right: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  z-index: 3;
  opacity: 0;
  transition: opacity 0.2s ease;
}
.photo-card:hover .photo-actions {
  opacity: 1;
}
.action-btn {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  color: #fff;
  font-size: 0.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}
.action-btn:hover {
  background: var(--accent);
  transform: scale(1.1);
}
.action-btn.delete:hover {
  background: #ef4444;
}

/* 图片容器 - 正方形（宽度自适应列宽） */
.photo-image {
  position: relative;
  width: 100%;
  aspect-ratio: 1 / 1;
  background: var(--bg-deep);
  overflow: hidden;
}
.photo-image img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

/* 日期覆盖层 */
.photo-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 20px 8px 6px;
  background: linear-gradient(to top, rgba(0,0,0,0.6), transparent);
  pointer-events: none;
}
.overlay-date {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.9);
  font-weight: 500;
  text-shadow: 0 1px 2px rgba(0,0,0,0.5);
}

.img-skeleton {
  position: absolute;
  inset: 0;
  background: var(--bg-deep);
  overflow: hidden;
}
.skeleton-shimmer {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent 0%, rgba(56, 189, 248, 0.08) 50%, transparent 100%);
  animation: shimmer 1.5s infinite;
}
@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

.img-fallback {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--bg-deep), var(--bg-deep));
}
.fallback-icon { font-size: 1.5rem; opacity: 0.4; }

.photo-info {
  padding: 6px 8px;
  border-top: 1px solid var(--border);
}

.photo-name {
  font-size: 0.68rem;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
}

.photo-meta {
  margin-top: 3px;
}

.meta-tag {
  font-size: 0.58rem;
  padding: 1px 5px;
  border-radius: 6px;
  display: inline-block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.meta-tag.located {
  background: var(--success-soft);
  color: var(--success);
}
.meta-tag.unlocated {
  background: var(--warning-soft);
  color: var(--warning);
}

.empty-state {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  width: 100%;
}
.empty-icon {
  font-size: 2.5rem;
  margin-bottom: 12px;
  opacity: 0.5;
}
.empty-text {
  font-size: 0.9rem;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.empty-hint {
  font-size: 0.75rem;
  color: var(--text-muted);
}

/* 位置编辑弹窗 */
.location-editor-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}
.location-editor-modal {
  width: 90%;
  max-width: 420px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}
.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}
.editor-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}
.editor-close {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  color: var(--text-muted);
  font-size: 0.8rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.editor-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.editor-body {
  padding: 16px;
  max-height: 60vh;
  overflow-y: auto;
}
.editor-desc {
  font-size: 0.78rem;
  color: var(--text-muted);
  margin-bottom: 14px;
}
.editor-field {
  margin-bottom: 14px;
}
.field-label {
  display: block;
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 6px;
  font-weight: 500;
}
.search-row {
  display: flex;
  gap: 8px;
}
.search-input {
  flex: 1;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.8rem;
  outline: none;
  transition: border-color 0.2s;
}
.search-input:focus {
  border-color: var(--accent);
}
.search-btn {
  padding: 8px 14px;
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: #fff;
  font-size: 0.78rem;
  font-weight: 500;
  white-space: nowrap;
}
.search-btn:hover:not(:disabled) {
  opacity: 0.9;
}
.search-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.search-results {
  margin-top: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  max-height: 180px;
  overflow-y: auto;
  background: var(--bg-card);
}
.search-result-item {
  padding: 8px 10px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  transition: background 0.15s;
}
.search-result-item:last-child {
  border-bottom: none;
}
.search-result-item:hover {
  background: var(--bg-hover);
}
.result-name {
  font-size: 0.78rem;
  color: var(--text-primary);
  font-weight: 500;
}
.result-address {
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-top: 2px;
}
.coord-row {
  display: flex;
  gap: 10px;
}
.coord-input {
  flex: 1;
}
.coord-label {
  display: block;
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-bottom: 4px;
}
.coord-input input {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.78rem;
  outline: none;
  transition: border-color 0.2s;
}
.coord-input input:focus {
  border-color: var(--accent);
}
.address-input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.8rem;
  outline: none;
  transition: border-color 0.2s;
}
.address-input:focus {
  border-color: var(--accent);
}
.editor-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
}
.cancel-btn {
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.8rem;
  border: 1px solid var(--border);
}
.cancel-btn:hover {
  background: var(--bg-hover);
}
.confirm-btn {
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: #fff;
  font-size: 0.8rem;
  font-weight: 500;
}
.confirm-btn:hover:not(:disabled) {
  opacity: 0.9;
}
.confirm-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
