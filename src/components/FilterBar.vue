<template>
  <div class="filter-bar">
    <div class="filter-group">
      <label>{{ $t("filter.startDate") }}</label>
      <input type="date" v-model="store.filter.startDate" />
    </div>
    <div class="filter-group">
      <label>{{ $t("filter.endDate") }}</label>
      <input type="date" v-model="store.filter.endDate" />
    </div>
    <div class="filter-group">
      <label>{{ $t("filter.province") }}</label>
      <select v-model="store.filter.province" @change="onProvinceChange">
        <option :value="null">{{ $t("common.all") }}</option>
        <option v-for="p in provinces" :key="p" :value="p">{{ p }}</option>
      </select>
    </div>
    <div class="filter-group">
      <label>{{ $t("filter.city") }}</label>
      <select v-model="store.filter.city" :disabled="!store.filter.province">
        <option :value="null">{{ $t("common.all") }}</option>
        <option v-for="c in cities" :key="c" :value="c">{{ c }}</option>
      </select>
    </div>
    <div class="filter-group">
      <label>{{ $t("filter.keyword") }}</label>
      <input type="text" v-model="store.filter.keyword" :placeholder="$t('filter.keywordPlaceholder')" />
    </div>
    <button class="btn-ghost" @click="reset">{{ $t("common.reset") }}</button>
    <button class="btn-ghost" @click="emit('close')">✕</button>
  </div>
</template>

<script setup>
import { computed } from "vue";
import { usePhotoStore } from "../stores/photoStore";

const emit = defineEmits(["close"]);
const store = usePhotoStore();

const provinces = computed(() => Object.keys(store.treeByLocation));
const cities = computed(() => {
  if (!store.filter.province) return [];
  const provinceData = store.treeByLocation[store.filter.province];
  if (!provinceData) return [];
  const list = Object.keys(provinceData);
  // 地点树可能以区县/地址作为筛选值，下拉中补一项便于回显
  if (store.filter.city && !list.includes(store.filter.city)) {
    list.push(store.filter.city);
  }
  return list;
});

function onProvinceChange() {
  store.filter.city = null;
}

function reset() {
  store.resetFilter();
}
</script>

<style scoped>
.filter-bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  background: var(--bg2);
  border-bottom: 1px solid var(--rule);
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.filter-group label {
  font-size: 0.7rem;
  color: var(--muted);
}
.filter-group input,
.filter-group select {
  min-width: 120px;
}
</style>
