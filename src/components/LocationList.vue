<template>
  <div class="location-list">
    <div class="loc-header">
      <span class="loc-title">分类树（按地点）</span>
      <button v-if="hasActiveFilter" class="clear-btn" @click="clearFilter">清除</button>
    </div>

    <div class="loc-content">
      <div v-for="group in groupedByProvince" :key="group.province" class="loc-province">
        <div class="province-name" @click="toggleGroup(group.province)"
          :class="{ expanded: !collapsed.has(group.province) }">
          <span class="tree-arrow">{{ collapsed.has(group.province) ? '▸' : '▾' }}</span>
          <span class="province-icon">📍</span>
          <span>{{ group.province || "未知地区" }}</span>
          <span class="province-count">{{ group.totalCount }}</span>
        </div>
        <div v-show="!collapsed.has(group.province)" class="loc-cities">
          <div v-for="city in group.cities" :key="city.city" class="loc-city"
            :class="{ active: isCityActive(city) }"
            @click="onCityClick(city)">
            <span class="city-dot"></span>
            <span class="city-name">
              <span class="city-name-main">{{ city.city || "未知城市" }}</span>
              <span v-if="city.address && city.district && city.district !== city.city" class="city-name-sub">
                {{ city.district }}
              </span>
            </span>
            <span class="city-count">{{ city.count }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.locationCounts.length === 0 && store.photos.length > 0" class="empty-state">
        <div class="empty-icon">🗺️</div>
        <div class="empty-text">暂无地点信息</div>
        <div class="empty-hint">导入带GPS的照片或手动标注位置</div>
      </div>

      <div v-if="store.photos.length === 0" class="empty-state">
        <div class="empty-icon">📷</div>
        <div class="empty-text">暂无照片</div>
        <div class="empty-hint">导入照片后按地点浏览</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, watch, onMounted, ref } from "vue";
import { usePhotoStore } from "../stores/photoStore";

const store = usePhotoStore();
const emit = defineEmits(["focus-location"]);
const isGeocoding = ref(false);

// 分类树折叠状态
const collapsed = ref(new Set());

function toggleGroup(province) {
  if (collapsed.value.has(province)) {
    collapsed.value.delete(province);
  } else {
    collapsed.value.add(province);
  }
}

// 检测是否有坐标地点需要解析
const hasCoordLocations = computed(() => {
  return store.locationCounts.some(loc => loc.province === "坐标地点");
});

// 自动触发逆地理编码
async function autoGeocode() {
  if (!hasCoordLocations.value || isGeocoding.value || store.geocodeProgress) return;
  isGeocoding.value = true;
  try {
    await store.runBatchGeocode();
  } catch (e) {
    // 静默失败，可能是未配置API Key
  } finally {
    isGeocoding.value = false;
  }
}

onMounted(() => {
  autoGeocode();
});

watch(hasCoordLocations, (newVal) => {
  if (newVal) autoGeocode();
});

const groupedByProvince = computed(() => {
  const map = new Map();
  for (const loc of store.locationCounts) {
    const prov = loc.province || "未知地区";
    if (!map.has(prov)) {
      map.set(prov, { province: prov, totalCount: 0, cities: [] });
    }
    const group = map.get(prov);
    group.totalCount += loc.count;
    group.cities.push({
      city: loc.city,
      count: loc.count,
      latitude: loc.latitude,
      longitude: loc.longitude,
      province: loc.province,
      district: loc.district,
      address: loc.address,
    });
  }
  return Array.from(map.values()).sort((a, b) => b.totalCount - a.totalCount);
});

const hasActiveFilter = computed(() => {
  return store.filter.city || store.filter.lat != null;
});

function isCityActive(city) {
  if (store.filter.city === city.city && store.filter.province === city.province) return true;
  if (store.filter.lat != null && city.latitude &&
      Math.abs(store.filter.lat - city.latitude) < 0.01) return true;
  return false;
}

function onCityClick(city) {
  // 如果点击的是坐标分组（province为"坐标地点"），用坐标筛选
  if (city.province === "坐标地点" && city.latitude != null) {
    if (store.filter.lat != null && Math.abs(store.filter.lat - city.latitude) < 0.01) {
      // 取消筛选
      store.setFilter({ lat: null, lng: null, city: null, province: null });
    } else {
      store.setFilter({
        lat: city.latitude,
        lng: city.longitude,
        city: null,
        province: null,
      });
    }
  } else {
    // 省市区筛选
    if (store.filter.city === city.city) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({
        province: city.province,
        city: city.city,
        lat: null,
        lng: null,
      });
    }
  }

  // 同时在地图上定位
  if (city.latitude != null && city.longitude != null) {
    emit("focus-location", { lat: city.latitude, lng: city.longitude });
  }
}

function clearFilter() {
  store.setFilter({ province: null, city: null, lat: null, lng: null });
}
</script>

<style scoped>
.location-list {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.loc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
}
.loc-title {
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

.loc-content {
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 12px;
}

.loc-province {
  margin-bottom: 16px;
}

.province-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
}
.province-name:hover {
  color: var(--accent);
}
.tree-arrow {
  font-size: 0.7rem;
  color: var(--text-muted);
  width: 12px;
  transition: transform 0.2s ease;
}
.province-icon {
  font-size: 0.85rem;
}
.province-count {
  margin-left: auto;
  font-size: 0.7rem;
  color: var(--text-muted);
  font-weight: 400;
}

.loc-cities {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.loc-city {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
}
.loc-city:hover {
  background: var(--bg-hover);
}
.loc-city.active {
  background: rgba(14, 165, 233, 0.08);
}
.loc-city.active .city-dot {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}
.loc-city.active .city-name {
  color: var(--accent);
  font-weight: 600;
}

.city-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0.5;
}

.city-name {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.city-name-main {
  font-size: 0.82rem;
  color: var(--text-secondary);
}
.city-name-sub {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.city-count {
  font-size: 0.72rem;
  color: var(--text-muted);
  background: var(--bg-card);
  padding: 2px 7px;
  border-radius: 10px;
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
