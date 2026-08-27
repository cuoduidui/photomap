<template>
  <div class="timeline">
    <div class="tl-header">
      <span class="tl-title">按时间浏览</span>
      <button v-if="activeYear || activeMonth" class="clear-btn" @click="clearFilter">
        清除筛选
      </button>
    </div>

    <div class="tl-content">
      <div v-for="group in groupedByYear" :key="group.year" class="tl-year-group">
        <div class="tl-year" @click="toggleYear(group.year)">
          <span class="year-arrow" :class="{ open: expandedYears.has(group.year) }">▶</span>
          <span class="year-badge">{{ group.year }}</span>
          <span class="year-count">{{ group.count }} 张</span>
        </div>
        <div v-if="expandedYears.has(group.year)" class="tl-months">
          <div v-for="month in group.months" :key="month.month" class="tl-month"
            :class="{ active: activeYear === group.year && activeMonth === month.month }"
            @click="onMonthClick(group.year, month.month)">
            <div class="month-dot"></div>
            <div class="month-info">
              <span class="month-name">{{ month.month }}月</span>
            </div>
            <span class="month-count">{{ month.count }} 张</span>
          </div>
        </div>
      </div>

      <div v-if="store.photos.length === 0" class="empty-state">
        <div class="empty-icon">📅</div>
        <div class="empty-text">暂无照片</div>
        <div class="empty-hint">导入照片后按时间线浏览</div>
      </div>

      <div v-else-if="store.photos.length > 0 && groupedByYear.length === 0" class="empty-state">
        <div class="empty-icon">📷</div>
        <div class="empty-text">无拍摄时间</div>
        <div class="empty-hint">这些照片没有EXIF拍摄时间信息</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { usePhotoStore } from "../stores/photoStore";

const store = usePhotoStore();
const expandedYears = ref(new Set());
const activeYear = ref(null);
const activeMonth = ref(null);

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
    const startDate = `${year}-${padMonth}-01T00:00:00`;
    const endDate = `${year}-${padMonth}-${String(lastDay).padStart(2, '0')}T23:59:59`;
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
