<template>
  <div class="location-list">
    <div class="loc-header">
      <span class="loc-title">{{ $t("location.title") }}</span>
      <button v-if="hasActiveFilter" class="clear-btn" @click="clearFilter">{{ $t("location.clearFilter") }}</button>
    </div>

    <div class="loc-content">
      <LocationTreeNode
        v-for="node in locationTree"
        :key="node.key"
        :node="node"
        :active-key="activeKey"
        @select="onSelect" />

      <div v-if="locationTree.length === 0 && store.photos.length > 0" class="empty-state">
        <div class="empty-icon">🗺️</div>
        <div class="empty-text">{{ $t("location.noLocationInfo") }}</div>
        <div class="empty-hint">{{ $t("location.noLocationHint") }}</div>
      </div>

      <div v-if="store.photos.length === 0" class="empty-state">
        <div class="empty-icon">📷</div>
        <div class="empty-text">{{ $t("location.noPhotos") }}</div>
        <div class="empty-hint">{{ $t("location.noPhotosHint") }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, watch, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import LocationTreeNode from "./LocationTreeNode.vue";

const store = usePhotoStore();
const { t } = useI18n();
const emit = defineEmits(["focus-location"]);
const isGeocoding = ref(false);

// 直辖市：省名即城市名，区县直接挂在省下面
const MUNICIPALITIES = new Set(["北京市", "上海市", "天津市", "重庆市"]);
// 街道/乡镇名提取：地址去掉省市区前缀后，取开头的 乡镇/街道 等
const STREET_RE = /^([\u4e00-\u9fa5]{2,10}?(?:街道|镇|乡|开发区|园区|大街|街|路|胡同|巷|村|社区|园))/;

function cleanShort(s) {
  return s.replace(/^[·,，、\s]+/, "");
}

// 去掉地址中与上级重复的省市区前缀
function shortAddress(province, city, district, address) {
  if (!address) return "";
  const parts = [province, city];
  if (district && district !== city) parts.push(district);
  const prefix = parts.filter(Boolean).join("");
  let short = address;
  if (prefix && short.startsWith(prefix)) {
    short = short.slice(prefix.length);
  }
  return cleanShort(short) || address;
}

function extractStreet(short) {
  const m = short.match(STREET_RE);
  return m ? m[1] : "";
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

// 从照片原始字段构建 省 → 市 → 区县 → 乡镇/街道 → 地址 树
const locationTree = computed(() => {
  const rootMap = new Map();

  function ensure(map, key) {
    if (!map.has(key)) {
      map.set(key, { count: 0, latSum: 0, lngSum: 0, children: new Map(), meta: {} });
    }
    return map.get(key);
  }

  for (const p of store.photos) {
    let province, isMun = false, city, district, address, coord = null;
    if (p.province) {
      province = p.province;
      isMun = MUNICIPALITIES.has(province);
      city = isMun ? province : (p.city || "未知城市");
      district = isMun
        ? (p.city && p.city !== province ? p.city : (p.district || ""))
        : (p.district && p.district !== city ? p.district : "");
      address = p.address || "";
    } else if (p.latitude != null && p.longitude != null) {
      province = "坐标地点";
      coord = {
        lat: Math.round(p.latitude * 100) / 100,
        lng: Math.round(p.longitude * 100) / 100,
      };
    } else {
      continue;
    }

    const path = [province];
    const kinds = ["province"];
    if (coord) {
      path.push(`坐标 ${coord.lat}, ${coord.lng}`);
      kinds.push("coord");
    } else {
      if (!isMun) {
        path.push(city);
        kinds.push("city");
      }
      if (district) {
        path.push(district);
        kinds.push("district");
      }
      if (address) {
        const rawCity = isMun ? (p.city || province) : (p.city || "未知城市");
        const short = shortAddress(province, rawCity, district, address);
        const street = extractStreet(short);
        if (street) {
          path.push(street);
          kinds.push("street");
        }
        path.push(address);
        kinds.push("address");
      }
    }

    const rawCity = isMun ? (p.city || province) : (p.city || "未知城市");
    let level = rootMap;
    for (let i = 0; i < path.length; i++) {
      const node = ensure(level, path[i]);
      node.meta.kind = kinds[i];
      node.meta.province = province;
      if (coord && kinds[i] === "coord") node.meta.coord = coord;
      if (!coord && address && kinds[i] === "address") {
        node.meta.addrFull = shortAddress(province, rawCity, district, address);
        const street = extractStreet(node.meta.addrFull);
        node.meta.street = street;
        node.meta.addrStrip = street
          ? cleanShort(node.meta.addrFull.slice(street.length))
          : node.meta.addrFull;
      }
      node.count += 1;
      if (p.latitude != null) {
        node.latSum += p.latitude;
        node.lngSum += p.longitude;
      }
      level = node.children;
    }
  }

  function convert(map, keyPrefix) {
    const nodes = [];
    for (const [name, v] of map) {
      const m = v.meta || {};
      const key = `${keyPrefix}|${name}`;
      const children = convert(v.children, key);
      if (m.kind === "street" && children.length < 2) {
        // 单个地址的街道不单独成组，地址直接上抛到上一级
        for (const child of children) {
          child.name = child.addrFull || child.name;
          nodes.push(child);
        }
        continue;
      }
      const kind = m.kind || "unknown";
      const displayName = name === "坐标地点"
        ? t("location.coordPlaces")
        : name === "未知城市"
          ? t("location.unknownCity")
          : kind === "coord" && name.startsWith("坐标 ")
            ? t("location.coordLabel", { lat: name.slice(3).split(", ")[0], lng: name.slice(3).split(", ")[1] })
            : name;
      nodes.push({
        kind,
        key,
        rawName: name,
        name: kind === "address" ? (m.addrStrip || m.addrFull || name) : displayName,
        title: kind === "address" ? (m.addrFull || name) : displayName,
        province: m.province || name,
        provinceOnly: kind === "city" && name === "未知城市",
        count: v.count,
        lat: v.latSum / v.count,
        lng: v.lngSum / v.count,
        coord: m.coord || null,
        addrFull: m.addrFull || null,
        children,
      });
    }
    nodes.sort((a, b) => b.count - a.count);
    return nodes;
  }

  return convert(rootMap, "");
});

const hasActiveFilter = computed(() => {
  return store.filter.province || store.filter.city || store.filter.lat != null;
});

// 当前激活筛选对应的树节点 key（含祖先高亮）
const activeKey = computed(() => {
  const f = store.filter;
  function walk(nodes) {
    for (const n of nodes) {
      if (f.lat != null) {
        if (n.kind === "coord" && n.coord &&
            Math.abs(f.lat - n.coord.lat) < 0.01 && Math.abs(f.lng - n.coord.lng) < 0.01) {
          return n.key;
        }
      } else if (f.province) {
        if (n.province !== f.province) {
          const found = walk(n.children || []);
          if (found) return found;
          continue;
        }
        if (f.city == null) {
          if (n.kind === "province") return n.key;
        } else if (n.kind !== "province" && n.kind !== "coord") {
          const val = n.kind === "address" ? n.addrFull : n.rawName;
          if (val === f.city) return n.key;
        }
      }
      const found = walk(n.children || []);
      if (found) return found;
    }
    return null;
  }
  return walk(locationTree.value);
});

function zoomForKind(kind) {
  if (kind === "province") return 8;
  if (kind === "city") return 11;
  if (kind === "district") return 12;
  return 15; // street / address / coord
}

function onSelect(node) {
  if (node.kind === "province") {
    if (node.province === "坐标地点") {
      const first = node.children && node.children[0];
      if (first && first.lat != null) {
        emit("focus-location", { lat: first.lat, lng: first.lng, zoom: 15 });
      }
      return;
    }
    if (store.filter.province === node.province && !store.filter.city && store.filter.lat == null) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({ province: node.province, city: null, lat: null, lng: null });
    }
  } else if (node.kind === "coord") {
    if (store.filter.lat != null && Math.abs(store.filter.lat - node.coord.lat) < 0.01) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({ province: null, city: null, lat: node.coord.lat, lng: node.coord.lng });
    }
  } else {
    const cityVal = node.kind === "address" ? node.addrFull : (node.provinceOnly ? null : node.rawName);
    if (store.filter.province === node.province && store.filter.city === cityVal) {
      store.setFilter({ province: null, city: null, lat: null, lng: null });
    } else {
      store.setFilter({ province: node.province, city: cityVal, lat: null, lng: null });
    }
  }

  // 点击任何地理位置都让地图移动到该位置
  if (node.lat != null && node.lng != null) {
    emit("focus-location", { lat: node.lat, lng: node.lng, zoom: zoomForKind(node.kind) });
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
  padding: 0 10px 12px;
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
