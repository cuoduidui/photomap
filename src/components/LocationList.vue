<template>
  <div class="location-list">
    <div class="loc-header">
      <span class="loc-title">地点分类树（省 → 市 → 区县 → 地址）</span>
      <button v-if="hasActiveFilter" class="clear-btn" @click="clearFilter">清除筛选</button>
    </div>

    <div class="loc-content">
      <div v-for="group in locationTree" :key="group.province" class="loc-province">
        <div class="province-name" :class="{ expanded: !isCollapsed(provKey(group.province)), active: isProvinceActive(group) }"
          @click="toggleGroup(provKey(group.province))">
          <span class="tree-arrow">{{ isCollapsed(provKey(group.province)) ? '▸' : '▾' }}</span>
          <span class="province-icon">📍</span>
          <span>{{ group.province }}</span>
          <span class="province-count">{{ group.totalCount }}</span>
        </div>

        <div v-show="!isCollapsed(provKey(group.province))" class="loc-level level-2">
          <div v-for="city in group.cities" :key="city.name" class="loc-city">
            <div class="loc-row" :class="{ active: isCityActive(group, city) }">
              <span v-if="city.children.length" class="tree-arrow arrow-btn"
                @click.stop="toggleGroup(cityKey(group.province, city.name))">
                {{ isCollapsed(cityKey(group.province, city.name)) ? '▸' : '▾' }}
              </span>
              <span v-else class="tree-arrow arrow-placeholder"></span>
              <span class="node-dot"></span>
              <span class="node-name" @click="onCityClick(group, city)">{{ city.name }}</span>
              <span class="node-count">{{ city.count }}</span>
            </div>

            <div v-if="city.children.length" v-show="!isCollapsed(cityKey(group.province, city.name))" class="loc-level level-3">
              <div v-for="child in city.children" :key="child.kind + '|' + child.name" class="loc-sub">
                <template v-if="child.kind === 'district'">
                  <div class="loc-row" :class="{ active: isDistrictActive(child) }">
                    <span v-if="child.children.length" class="tree-arrow arrow-btn"
                      @click.stop="toggleGroup(distKey(group.province, city.name, child.name))">
                      {{ isCollapsed(distKey(group.province, city.name, child.name)) ? '▸' : '▾' }}
                    </span>
                    <span v-else class="tree-arrow arrow-placeholder"></span>
                    <span class="node-dot district-dot"></span>
                    <span class="node-name" @click="onDistrictClick(group, city, child)">{{ child.name }}</span>
                    <span class="node-count">{{ child.count }}</span>
                  </div>
                  <div v-if="child.children.length"
                    v-show="!isCollapsed(distKey(group.province, city.name, child.name))" class="loc-level level-4">
                    <div v-for="addr in child.children" :key="addr.address" class="loc-address"
                      :class="{ active: isAddressActive(addr) }" @click="onAddressClick(group, city, child, addr)">
                      <span class="address-dot"></span>
                      <span class="node-name address-name" :title="addr.fullName">{{ addr.name }}</span>
                      <span class="node-count">{{ addr.count }}</span>
                    </div>
                  </div>
                </template>
                <template v-else>
                  <div class="loc-address" :class="{ active: isAddressActive(child) }"
                    @click="onAddressClick(group, city, null, child)">
                    <span class="address-dot"></span>
                    <span class="node-name address-name" :title="child.fullName">{{ child.name }}</span>
                    <span class="node-count">{{ child.count }}</span>
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="locationTree.length === 0 && store.photos.length > 0" class="empty-state">
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

// 各级折叠状态
const collapsed = ref(new Set());

function provKey(province) { return `prov:${province}`; }
function cityKey(province, city) { return `city:${province}|${city}`; }
function distKey(province, city, district) { return `dist:${province}|${city}|${district}`; }

function isCollapsed(key) { return collapsed.value.has(key); }

function toggleGroup(key) {
  if (collapsed.value.has(key)) {
    collapsed.value.delete(key);
  } else {
    collapsed.value.add(key);
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

// 去掉地址中与上级重复的省市区前缀，仅保留街道级以下部分
function shortAddress(province, city, district, address) {
  if (!address) return "";
  const parts = [province, city];
  if (district && district !== city) parts.push(district);
  const prefix = parts.filter(Boolean).join("");
  let short = address;
  if (prefix && short.startsWith(prefix)) {
    short = short.slice(prefix.length);
  }
  short = short.replace(/^[·,，、\s]+/, "");
  return short || address;
}

// 从照片原始字段构建 省 → 市 → 区县 → 地址 四层树
const locationTree = computed(() => {
  const provMap = new Map();

  for (const p of store.photos) {
    let province, city, district, address;
    if (p.province) {
      province = p.province;
      city = p.city || "未知城市";
      district = p.district && p.district !== city ? p.district : "";
      address = p.address || "";
    } else if (p.latitude != null && p.longitude != null) {
      province = "坐标地点";
      city = `坐标 ${p.latitude.toFixed(2)}, ${p.longitude.toFixed(2)}`;
      district = "";
      address = "";
    } else {
      continue;
    }

    if (!provMap.has(province)) {
      provMap.set(province, { province, totalCount: 0, cities: new Map() });
    }
    const prov = provMap.get(province);
    prov.totalCount += 1;

    if (!prov.cities.has(city)) {
      prov.cities.set(city, {
        name: city,
        province,
        count: 0,
        latSum: 0,
        lngSum: 0,
        districts: new Map(),
        addresses: new Map(),
      });
    }
    const c = prov.cities.get(city);
    c.count += 1;
    if (p.latitude != null) { c.latSum += p.latitude; c.lngSum += p.longitude; }

    if (district) {
      if (!c.districts.has(district)) {
        c.districts.set(district, {
          name: district,
          province,
          city,
          count: 0,
          latSum: 0,
          lngSum: 0,
          addresses: new Map(),
        });
      }
      const d = c.districts.get(district);
      d.count += 1;
      if (p.latitude != null) { d.latSum += p.latitude; d.lngSum += p.longitude; }
      if (address) {
        if (!d.addresses.has(address)) {
          d.addresses.set(address, {
            kind: "address",
            name: shortAddress(province, city, district, address),
            fullName: address,
            province,
            city,
            district,
            address,
            count: 0,
            latSum: 0,
            lngSum: 0,
          });
        }
        const a = d.addresses.get(address);
        a.count += 1;
        if (p.latitude != null) { a.latSum += p.latitude; a.lngSum += p.longitude; }
      }
    } else if (address) {
      if (!c.addresses.has(address)) {
        c.addresses.set(address, {
          kind: "address",
          name: shortAddress(province, city, "", address),
          fullName: address,
          province,
          city,
          district: "",
          address,
          count: 0,
          latSum: 0,
          lngSum: 0,
        });
      }
      const a = c.addresses.get(address);
      a.count += 1;
      if (p.latitude != null) { a.latSum += p.latitude; a.lngSum += p.longitude; }
    }
  }

  const result = [];
  for (const prov of provMap.values()) {
    const cities = [];
    for (const c of prov.cities.values()) {
      const children = [];
      for (const d of c.districts.values()) {
        const addresses = Array.from(d.addresses.values()).sort((x, y) => y.count - x.count);
        children.push({
          kind: "district",
          name: d.name,
          province: d.province,
          city: d.city,
          count: d.count,
          lat: d.latSum / d.count,
          lng: d.lngSum / d.count,
          children: addresses,
        });
      }
      for (const a of c.addresses.values()) {
        children.push({
          ...a,
          lat: a.latSum / a.count,
          lng: a.lngSum / a.count,
        });
      }
      children.sort((x, y) => y.count - x.count);
      cities.push({
        name: c.name,
        province: c.province,
        count: c.count,
        lat: c.latSum / c.count,
        lng: c.lngSum / c.count,
        children,
      });
    }
    cities.sort((x, y) => y.count - x.count);
    result.push({ province: prov.province, totalCount: prov.totalCount, cities });
  }
  result.sort((x, y) => y.totalCount - x.totalCount);
  return result;
});

const hasActiveFilter = computed(() => {
  return store.filter.province || store.filter.city || store.filter.lat != null;
});

// 当前激活的筛选路径
const active = computed(() => {
  const f = store.filter;
  if (f.lat != null) return { type: "coord", lat: f.lat, lng: f.lng };
  if (f.province) return { type: f.city ? "named" : "province", province: f.province, value: f.city };
  return null;
});

function isProvinceActive(group) {
  return active.value && active.value.province === group.province;
}

function isAddressActive(addr) {
  const a = active.value;
  return !!a && a.type === "named" && a.province === addr.province && a.value === addr.address;
}

function isDistrictActive(district) {
  const a = active.value;
  if (!!a && a.type === "named" && a.province === district.province && a.value === district.name) return true;
  return district.children.some((c) => c.kind === "address" && isAddressActive(c));
}

function isCityActive(group, city) {
  const a = active.value;
  if (group.province === "坐标地点") {
    return !!a && a.type === "coord" &&
      Math.abs(a.lat - city.lat) < 0.01 && Math.abs(a.lng - city.lng) < 0.01;
  }
  if (city.name === "未知城市") {
    return !!a && a.type === "province" && a.province === group.province;
  }
  if (!!a && a.type === "named" && a.province === city.province && a.value === city.name) return true;
  return city.children.some((c) => (c.kind === "district" ? isDistrictActive(c) : isAddressActive(c)));
}

function onCityClick(group, city) {
  if (group.province === "坐标地点") {
    if (store.filter.lat != null && Math.abs(store.filter.lat - city.lat) < 0.01) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({ province: null, city: null, lat: city.lat, lng: city.lng });
      emit("focus-location", { lat: city.lat, lng: city.lng });
    }
    return;
  }
  if (city.name === "未知城市") {
    if (store.filter.province === group.province && !store.filter.city) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({ province: group.province, city: null, lat: null, lng: null });
      if (city.lat != null) emit("focus-location", { lat: city.lat, lng: city.lng });
    }
    return;
  }
  if (store.filter.province === group.province && store.filter.city === city.name) {
    store.setFilter({ province: null, city: null, lat: null, lng: null });
  } else {
    store.setFilter({ province: group.province, city: city.name, lat: null, lng: null });
    if (city.lat != null) emit("focus-location", { lat: city.lat, lng: city.lng });
  }
}

function onDistrictClick(group, city, district) {
  if (store.filter.province === group.province && store.filter.city === district.name) {
    store.setFilter({ province: null, city: null, lat: null, lng: null });
  } else {
    store.setFilter({ province: group.province, city: district.name, lat: null, lng: null });
    if (district.lat != null) emit("focus-location", { lat: district.lat, lng: district.lng });
  }
}

function onAddressClick(group, city, district, addr) {
  if (store.filter.province === group.province && store.filter.city === addr.address) {
    store.setFilter({ province: null, city: null, lat: null, lng: null });
  } else {
    store.setFilter({ province: group.province, city: addr.address, lat: null, lng: null });
    if (addr.lat != null) emit("focus-location", { lat: addr.lat, lng: addr.lng });
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
  font-size: 0.78rem;
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
  margin-bottom: 14px;
}

.province-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 6px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
}
.province-name:hover,
.province-name.active {
  color: var(--accent);
}
.tree-arrow {
  font-size: 0.7rem;
  color: var(--text-muted);
  width: 12px;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}
.arrow-btn {
  cursor: pointer;
}
.arrow-placeholder {
  visibility: hidden;
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

.loc-level {
  margin-left: 12px;
  padding-left: 8px;
  border-left: 1px solid var(--border);
}
.level-2 {
  margin-left: 0;
  padding-left: 0;
  border-left: none;
}

.loc-city {
  margin-bottom: 2px;
}

.loc-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
}
.loc-row:hover {
  background: var(--bg-hover);
}
.loc-row.active {
  background: rgba(14, 165, 233, 0.08);
}
.loc-row.active .node-dot {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}
.loc-row.active .node-name {
  color: var(--accent);
  font-weight: 600;
}

.node-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0.5;
  flex-shrink: 0;
}
.district-dot {
  width: 6px;
  height: 6px;
}
.node-name {
  flex: 1;
  font-size: 0.8rem;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.node-count {
  font-size: 0.7rem;
  color: var(--text-muted);
  background: var(--bg-card);
  padding: 2px 7px;
  border-radius: 10px;
  flex-shrink: 0;
}

.loc-address {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px 4px 16px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
}
.loc-address:hover {
  background: var(--bg-hover);
}
.loc-address.active {
  background: rgba(14, 165, 233, 0.08);
}
.loc-address.active .address-dot {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}
.loc-address.active .address-name {
  color: var(--accent);
  font-weight: 600;
}
.address-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0.4;
  flex-shrink: 0;
}
.address-name {
  font-size: 0.76rem;
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
