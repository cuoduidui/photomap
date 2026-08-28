<template>
  <div class="timeline">
    <div class="tl-header">
      <span class="tl-title">{{ $t("timeline.title") }}</span>
      <button v-if="activeYear || activeMonth" class="clear-btn" @click="clearFilter">
        {{ $t("timeline.clearFilter") }}
      </button>
    </div>

    <div class="tl-content">
      <div v-if="hasDatedPhotos" class="tl-range">
        <div class="range-header">
          <span class="range-label">{{ rangeLabel }}</span>
          <span class="range-count">{{ $t("timeline.photosCount", { n: rangeCount }) }}</span>
          <button v-if="isRangeFiltered" class="range-reset" @click="resetRange">{{ $t("common.reset") }}</button>
        </div>
        <div class="range-hint">{{ $t("timeline.rangeHint") }}</div>
        <div class="density-bar" :title="$t('timeline.clickDay')">
          <div v-for="(c, i) in dailyCounts" :key="i" class="density-col"
            :class="{ inrange: i >= startIdx && i <= endIdx, has: c > 0 }"
            :style="{ height: Math.max(8, (c / maxDaily * 100)) + '%' }"
            :title="$t('timeline.photosOnDate', { date: indexToDateStr(i), n: c })"
            @click="selectDay(i)"></div>
        </div>
        <div ref="trackRef" class="range-track" @pointerdown="onTrackDown">
          <div class="range-bar" :style="{ left: startPct + '%', width: Math.max(0, endPct - startPct) + '%' }"
            @pointerdown.stop="onRangeBarDown"></div>
          <div class="range-handle start" :style="{ left: startPct + '%' }"
            @pointerdown.stop="onHandleDown($event, 'start')"></div>
          <div class="range-handle end" :style="{ left: endPct + '%' }"
            @pointerdown.stop="onHandleDown($event, 'end')"></div>
        </div>
        <div class="range-dates">
          <span>{{ indexToDateStr(startIdx) }}</span>
          <span>{{ $t("timeline.to") }}</span>
          <span>{{ indexToDateStr(endIdx) }}</span>
        </div>
      </div>

      <div v-for="group in groupedByYear" :key="group.year" class="tl-year-group">
        <div class="tl-year" @click="toggleYear(group.year)">
          <span class="year-arrow" :class="{ open: expandedYears.has(group.year) }">▶</span>
          <span class="year-badge">{{ group.year }}</span>
          <span class="year-count">{{ $t("timeline.photosCount", { n: group.count }) }}</span>
        </div>
        <div v-if="expandedYears.has(group.year)" class="tl-months">
          <div v-for="month in group.months" :key="month.month" class="tl-month"
            :class="{ active: activeYear === group.year && activeMonth === month.month }"
            @click="onMonthClick(group.year, month.month)">
            <div class="month-dot"></div>
            <div class="month-info">
              <span class="month-name">{{ $t("timeline.month", { m: month.month }) }}</span>
            </div>
            <span class="month-count">{{ $t("timeline.photosCount", { n: month.count }) }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.photos.length === 0" class="empty-state">
        <div class="empty-icon">📅</div>
        <div class="empty-text">{{ $t("timeline.noPhotos") }}</div>
        <div class="empty-hint">{{ $t("timeline.noPhotosHint") }}</div>
      </div>

      <div v-else-if="store.photos.length > 0 && groupedByYear.length === 0" class="empty-state">
        <div class="empty-icon">📷</div>
        <div class="empty-text">{{ $t("timeline.noTime") }}</div>
        <div class="empty-hint">{{ $t("timeline.noTimeHint") }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onUnmounted } from "vue";
import { usePhotoStore } from "../stores/photoStore";

const store = usePhotoStore();
const expandedYears = ref(new Set());
const activeYear = ref(null);
const activeMonth = ref(null);

// ---------- 日期范围拖拽 ----------
const trackRef = ref(null);
const startIdx = ref(0);
const endIdx = ref(0);
let dragState = null;

function parseDate(s) {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

function toDateStr(dt) {
  const y = dt.getFullYear();
  const m = String(dt.getMonth() + 1).padStart(2, "0");
  const d = String(dt.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

const minDate = computed(() => {
  let mn = null;
  for (const p of store.photos) {
    if (!p.taken_time) continue;
    const s = p.taken_time.slice(0, 10);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) continue;
    if (!mn || s < mn) mn = s;
  }
  return mn;
});

const maxDate = computed(() => {
  let mx = null;
  for (const p of store.photos) {
    if (!p.taken_time) continue;
    const s = p.taken_time.slice(0, 10);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) continue;
    if (!mx || s > mx) mx = s;
  }
  return mx;
});

const dayCount = computed(() => {
  if (!minDate.value || !maxDate.value) return 0;
  return Math.round((parseDate(maxDate.value) - parseDate(minDate.value)) / 86400000) + 1;
});

const hasDatedPhotos = computed(() => dayCount.value > 0);

function indexToDateStr(idx) {
  if (!minDate.value) return "";
  const dt = parseDate(minDate.value);
  dt.setDate(dt.getDate() + idx);
  return toDateStr(dt);
}

function dateStrToIndex(s) {
  if (!minDate.value || !s) return 0;
  return Math.round((parseDate(s) - parseDate(minDate.value)) / 86400000);
}

const dailyCounts = computed(() => {
  const arr = new Array(dayCount.value).fill(0);
  for (const p of store.photos) {
    if (!p.taken_time) continue;
    const idx = dateStrToIndex(p.taken_time.slice(0, 10));
    if (idx >= 0 && idx < dayCount.value) arr[idx]++;
  }
  return arr;
});

const maxDaily = computed(() => Math.max(1, ...dailyCounts.value));

const startPct = computed(() => {
  if (dayCount.value <= 1) return 0;
  return (startIdx.value / (dayCount.value - 1)) * 100;
});

const endPct = computed(() => {
  if (dayCount.value <= 1) return 0;
  return (endIdx.value / (dayCount.value - 1)) * 100;
});

const rangeLabel = computed(() => {
  const s = indexToDateStr(startIdx.value);
  const e = indexToDateStr(endIdx.value);
  return s === e ? s : `${s} ~ ${e}`;
});

const rangeCount = computed(() => {
  const s = indexToDateStr(startIdx.value);
  const e = indexToDateStr(endIdx.value);
  const start = `${s} 00:00:00`;
  const end = `${e} 23:59:59`;
  return store.photos.filter((p) => p.taken_time && p.taken_time >= start && p.taken_time <= end).length;
});

const isRangeFiltered = computed(() => {
  return dayCount.value > 1 && (startIdx.value > 0 || endIdx.value < dayCount.value - 1);
});

function indexFromEvent(e) {
  const rect = trackRef.value.getBoundingClientRect();
  const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
  return Math.round(ratio * (dayCount.value - 1));
}

function applyRangeFromSlider() {
  activeYear.value = null;
  activeMonth.value = null;
  const s = indexToDateStr(startIdx.value);
  const e = indexToDateStr(endIdx.value);
  if (startIdx.value === 0 && endIdx.value === dayCount.value - 1) {
    store.setFilter({ startDate: null, endDate: null });
  } else {
    store.setFilter({ startDate: `${s} 00:00:00`, endDate: `${e} 23:59:59` });
  }
}

function onHandleDown(e, type) {
  e.preventDefault();
  dragState = { type, startIdx: startIdx.value, endIdx: endIdx.value, startX: e.clientX };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd);
}

function onRangeBarDown(e) {
  e.preventDefault();
  dragState = { type: "range", startIdx: startIdx.value, endIdx: endIdx.value, startX: e.clientX };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd);
}

function onTrackDown(e) {
  e.preventDefault();
  const idx = indexFromEvent(e);
  startIdx.value = idx;
  endIdx.value = idx;
  applyRangeFromSlider();
  dragState = { type: "start", startIdx: idx, endIdx: idx, startX: e.clientX };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd);
}

function onDragMove(e) {
  if (!dragState || dayCount.value <= 1) return;
  const idx = indexFromEvent(e);
  if (dragState.type === "start") {
    if (idx <= dragState.endIdx) {
      startIdx.value = idx;
    } else {
      startIdx.value = dragState.endIdx;
      endIdx.value = idx;
    }
  } else if (dragState.type === "end") {
    if (idx >= dragState.startIdx) {
      endIdx.value = idx;
    } else {
      endIdx.value = dragState.startIdx;
      startIdx.value = idx;
    }
  } else {
    const pxPerDay = trackRef.value.getBoundingClientRect().width / (dayCount.value - 1);
    const deltaDays = Math.round((e.clientX - dragState.startX) / pxPerDay);
    const span = dragState.endIdx - dragState.startIdx;
    const maxStart = dayCount.value - 1 - span;
    const ns = Math.max(0, Math.min(maxStart, dragState.startIdx + deltaDays));
    startIdx.value = ns;
    endIdx.value = ns + span;
  }
  applyRangeFromSlider();
}

function onDragEnd() {
  dragState = null;
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", onDragEnd);
}

function selectDay(i) {
  startIdx.value = i;
  endIdx.value = i;
  applyRangeFromSlider();
}

function resetRange() {
  startIdx.value = 0;
  endIdx.value = Math.max(0, dayCount.value - 1);
  applyRangeFromSlider();
}

// 外部修改日期筛选（如月份点击/重置）时同步滑块
function syncFromFilter() {
  if (dayCount.value <= 0) return;
  const s = store.filter.startDate;
  const e = store.filter.endDate;
  startIdx.value = s ? Math.max(0, Math.min(dayCount.value - 1, dateStrToIndex(s.slice(0, 10)))) : 0;
  endIdx.value = e ? Math.max(0, Math.min(dayCount.value - 1, dateStrToIndex(e.slice(0, 10)))) : dayCount.value - 1;
}

watch(
  () => [store.filter.startDate, store.filter.endDate],
  syncFromFilter,
  { immediate: true }
);

watch(dayCount, syncFromFilter);

onUnmounted(() => {
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", onDragEnd);
});

// ---------- 原有年/月树 ----------
const groupedByYear = computed(() => {
  const yearMap = new Map();

  for (const photo of store.photos) {
    if (!photo.taken_time) continue;
    const d = new Date(photo.taken_time);
    if (isNaN(d.getTime())) continue;
    const year = d.getFullYear();
    const month = d.getMonth() + 1;

    if (!yearMap.has(year)) {
      yearMap.set(year, { year, count: 0, months: new Map() });
    }
    const yearGroup = yearMap.get(year);
    yearGroup.count++;

    if (!yearGroup.months.has(month)) {
      yearGroup.months.set(month, { month, count: 0 });
    }
    yearGroup.months.get(month).count++;
  }

  return Array.from(yearMap.values())
    .sort((a, b) => b.year - a.year)
    .map(g => ({
      ...g,
      months: Array.from(g.months.values()).sort((a, b) => b.month - a.month),
    }));
});

function toggleYear(year) {
  if (expandedYears.value.has(year)) {
    expandedYears.value.delete(year);
  } else {
    expandedYears.value.add(year);
  }
  expandedYears.value = new Set(expandedYears.value);
}

function onMonthClick(year, month) {
  if (activeYear.value === year && activeMonth.value === month) {
    clearFilter();
  } else {
    activeYear.value = year;
    activeMonth.value = month;
    const padMonth = String(month).padStart(2, '0');
    const lastDay = new Date(year, month, 0).getDate();
    const startDate = `${year}-${padMonth}-01 00:00:00`;
    const endDate = `${year}-${padMonth}-${String(lastDay).padStart(2, '0')} 23:59:59`;
    store.setFilter({ startDate, endDate });
  }
}

function clearFilter() {
  activeYear.value = null;
  activeMonth.value = null;
  store.setFilter({ startDate: null, endDate: null });
}
</script>

<style scoped>
.timeline {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.tl-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
}
.tl-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}
.clear-btn {
  font-size: 0.7rem;
  color: var(--accent);
  padding: 2px 8px;
  border-radius: 10px;
  background: rgba(14, 165, 233, 0.08);
}
.clear-btn:hover {
  background: rgba(14, 165, 233, 0.15);
}

.tl-content {
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 12px;
}

.tl-range {
  margin-bottom: 16px;
  padding: 10px 12px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  user-select: none;
}
.range-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.range-label {
  flex: 1;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.range-count {
  font-size: 0.7rem;
  color: var(--accent);
  background: rgba(14, 165, 233, 0.08);
  padding: 2px 8px;
  border-radius: 10px;
}
.range-reset {
  font-size: 0.68rem;
  color: var(--accent);
  background: rgba(14, 165, 233, 0.08);
  padding: 2px 8px;
  border-radius: 10px;
}
.range-reset:hover {
  background: rgba(14, 165, 233, 0.15);
}
.range-hint {
  font-size: 0.66rem;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.density-bar {
  display: flex;
  align-items: flex-end;
  gap: 1px;
  height: 34px;
  padding: 0 2px;
  margin-bottom: 2px;
  cursor: pointer;
}
.density-col {
  flex: 1;
  min-height: 2px;
  border-radius: 1px;
  background: var(--border);
  opacity: 0.35;
  transition: opacity 0.15s ease;
}
.density-col.has {
  background: var(--accent);
  opacity: 0.35;
}
.density-col.inrange {
  opacity: 1;
  background: linear-gradient(180deg, var(--accent-2), var(--accent));
}
.density-col:hover {
  opacity: 0.85;
}

.range-track {
  position: relative;
  height: 24px;
  margin: 0 2px;
  cursor: pointer;
  touch-action: none;
}
.range-track::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 3px;
  transform: translateY(-50%);
  background: var(--border);
  border-radius: 2px;
}
.range-bar {
  position: absolute;
  top: 50%;
  height: 5px;
  transform: translateY(-50%);
  background: linear-gradient(90deg, var(--accent), var(--accent-2));
  border-radius: 3px;
  cursor: grab;
}
.range-bar:active {
  cursor: grabbing;
}
.range-handle {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  border: 3px solid var(--accent);
  box-shadow: 0 0 6px var(--accent-glow);
  cursor: ew-resize;
  z-index: 2;
}
.range-dates {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.66rem;
  color: var(--text-muted);
  margin-top: 2px;
}

.tl-year-group {
  margin-bottom: 16px;
}

.tl-year {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}
.year-arrow {
  font-size: 0.6rem;
  color: var(--text-muted);
  transition: transform 0.2s ease;
  width: 12px;
}
.year-arrow.open {
  transform: rotate(90deg);
}

.year-badge {
  font-size: 1rem;
  font-weight: 700;
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.year-count {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.tl-months {
  position: relative;
  padding-left: 14px;
}
.tl-months::before {
  content: '';
  position: absolute;
  left: 4px;
  top: 0;
  bottom: 0;
  width: 2px;
  background: linear-gradient(to bottom, var(--accent), transparent);
  opacity: 0.3;
}

.tl-month {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.15s ease;
  position: relative;
}
.tl-month:hover {
  background: var(--bg-hover);
}
.tl-month.active {
  background: rgba(99, 102, 241, 0.08);
}
.tl-month.active .month-dot {
  background: var(--accent-2);
  box-shadow: 0 0 8px var(--accent-2-glow);
}
.tl-month.active .month-name {
  color: var(--accent-2);
  font-weight: 600;
}

.month-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #fff;
  border: 2px solid var(--accent);
  z-index: 1;
  box-shadow: 0 0 4px var(--accent-glow);
}

.month-info {
  flex: 1;
}

.month-name {
  font-size: 0.85rem;
  color: var(--text-primary);
}

.month-count {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--text-muted);
}
.empty-icon { font-size: 2rem; margin-bottom: 10px; opacity: 0.5; }
.empty-text { font-size: 0.85rem; color: var(--text-secondary); margin-bottom: 4px; }
.empty-hint { font-size: 0.75rem; color: var(--text-muted); text-align: center; }
</style>
