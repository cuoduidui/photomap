<template>
  <div class="filter-bar">
    <div class="filter-group">
      <label>开始日期</label>
      <input type="date" v-model="store.filter.startDate" />
    </div>
    <div class="filter-group">
      <label>结束日期</label>
      <input type="date" v-model="store.filter.endDate" />
    </div>
    <div class="filter-group">
      <label>省份</label>
      <select v-model="store.filter.province" @change="onProvinceChange">
        <option :value="null">全部</option>
        <option v-for="p in provinces" :key="p" :value="p">{{ p }}</option>
      </select>
    </div>
    <div class="filter-group">
      <label>城市</label>
      <select v-model="store.filter.city" :disabled="!store.filter.province">
        <option :value="null">全部</option>
        <option v-for="c in cities" :key="c" :value="c">{{ c }}</option>
      </select>
    </div>
    <div class="filter-group">
      <label>关键词</label>
      <input type="text" v-model="store.filter.keyword" placeholder="文件名/地址" />
    </div>
    <button class="btn-ghost" @click="reset">重置</button>
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
  return Object.keys(provinceData);
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
