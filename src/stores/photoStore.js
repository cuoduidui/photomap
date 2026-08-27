import { defineStore } from "pinia";
import * as api from "../utils/tauri";
import { getAllPhotoTags, getAllTrips, autoClusterTrips, analyzeFaces, updateTag, onFaceAnalyzeProgress, getTagPhotos } from "../utils/tauri";

export const usePhotoStore = defineStore("photos", {
  state: () => ({
    photos: [],
    stats: { total: 0, located: 0, unlocated: 0, cities: 0 },
    locationCounts: [],
    customLocations: [],
    tags: [],
    photoTagMap: {},
    trips: [],
    selectedPhotoIds: [],
    filter: {
      startDate: null,
      endDate: null,
      province: null,
      city: null,
      lat: null,
      lng: null,
      tagIds: [],
      keyword: "",
    },
    loading: false,
    importProgress: null,
    geocodeProgress: null,
    apiConfigured: false,
    currentZoom: 5,
  }),

  getters: {
    filteredPhotos(state) {
      let result = state.photos;

      if (state.filter.startDate) {
        result = result.filter(
          (p) => p.taken_time && p.taken_time >= state.filter.startDate
        );
      }
      if (state.filter.endDate) {
        result = result.filter(
          (p) => p.taken_time && p.taken_time <= state.filter.endDate
        );
      }
      if (state.filter.province) {
        result = result.filter((p) => p.province === state.filter.province);
      }
      if (state.filter.city) {
        // 城市字段可能是街道级展示名（区县·街道），需同时匹配原 city 或组合名
        result = result.filter((p) => {
          if (p.city === state.filter.city) return true;
          const combo = [p.district, p.address].filter(Boolean).join("·");
          return combo === state.filter.city;
        });
      }
      if (state.filter.lat != null && state.filter.lng != null) {
        const lat = state.filter.lat;
        const lng = state.filter.lng;
        result = result.filter((p) =>
          p.latitude != null && p.longitude != null &&
          Math.abs(p.latitude - lat) < 0.01 &&
          Math.abs(p.longitude - lng) < 0.01
        );
      }
      if (state.filter.tagIds && state.filter.tagIds.length > 0) {
        result = result.filter((p) => {
          const photoTags = state.photoTagMap[p.id] || [];
          return state.filter.tagIds.every((tid) => photoTags.includes(tid));
        });
      }
      if (state.filter.keyword) {
        const kw = state.filter.keyword.toLowerCase();
        result = result.filter(
          (p) =>
            p.file_name.toLowerCase().includes(kw) ||
            (p.address && p.address.toLowerCase().includes(kw))
        );
      }
      return result;
    },

    locatedPhotos(state) {
      return state.photos.filter((p) => p.latitude != null && p.longitude != null);
    },

    unlocatedPhotos(state) {
      return state.photos.filter((p) => p.latitude == null || p.longitude == null);
    },

    personTags(state) {
      return state.tags.filter((t) => t.tag_type === "person");
    },

    customTags(state) {
      return state.tags.filter((t) => t.tag_type === "custom");
    },

    treeByLocation(state) {
      const tree = {};
      for (const p of state.photos) {
        if (!p.province) continue;
        if (!tree[p.province]) tree[p.province] = {};
        const city = p.city || "未知";
        if (!tree[p.province][city]) tree[p.province][city] = [];
        tree[p.province][city].push(p);
      }
      return tree;
    },

    treeByDate(state) {
      const tree = {};
      for (const p of state.photos) {
        if (!p.taken_time) continue;
        const year = p.taken_time.substring(0, 4);
        const yearMonth = p.taken_time.substring(0, 7);
        if (!tree[year]) tree[year] = {};
        if (!tree[year][yearMonth]) tree[year][yearMonth] = [];
        tree[year][yearMonth].push(p);
      }
      return tree;
    },
  },

  actions: {
    async init() {
      await Promise.all([
        this.loadPhotos(),
        this.loadStats(),
        this.loadLocationCounts(),
        this.loadCustomLocations(),
        this.loadTags(),
        this.checkApiConfig(),
        this.loadTrips(),
      ]);
      await this.loadAllPhotoTags();
    },

    async loadTrips() {
      try {
        this.trips = await getAllTrips();
      } catch (e) {
        console.warn("加载旅行失败:", e);
      }
    },

    async generateTrips() {
      try {
        this.trips = await autoClusterTrips();
      } catch (e) {
        console.error("生成旅行失败:", e);
        throw e;
      }
    },

    async loadPhotos() {
      this.loading = true;
      try {
        this.photos = await api.getAllPhotos();
      } finally {
        this.loading = false;
      }
    },

    async loadStats() {
      this.stats = await api.getStats();
    },

    async loadLocationCounts() {
      this.locationCounts = await api.getLocationCounts();
    },

    async loadCustomLocations() {
      this.customLocations = await api.getCustomLocations();
    },

    async loadTags() {
      this.tags = await api.getTags();
    },

    async loadAllPhotoTags() {
      const map = await api.getAllPhotoTags();
      for (const p of this.photos) {
        if (!map[p.id]) map[p.id] = [];
      }
      this.photoTagMap = map;
    },

    async analyzeFaces(onProgress) {
      let unlisten = null;
      if (onProgress) {
        unlisten = await onFaceAnalyzeProgress((done, total) => {
          onProgress(done, total);
        });
      }
      try {
        const result = await analyzeFaces();
        await this.loadTags();
        await this.loadAllPhotoTags();
        return result;
      } finally {
        if (unlisten) unlisten();
      }
    },

    async updateTag(id, name, description, faceThumb) {
      await updateTag(id, name, description, faceThumb);
      await this.loadTags();
    },

    async getTagPhotos(tagId) {
      return await getTagPhotos(tagId);
    },

    async createTag(name, tag_type = "custom", color = "#0ea5e9") {
      const tag = await api.createTag(name, tag_type, color);
      await this.loadTags();
      return tag;
    },

    async deleteTag(id) {
      await api.deleteTag(id);
      await this.loadTags();
      for (const pid of Object.keys(this.photoTagMap)) {
        this.photoTagMap[pid] = this.photoTagMap[pid].filter((tid) => tid !== id);
      }
    },

    async addTagToPhotos(photoIds, tagId) {
      for (const pid of photoIds) {
        try {
          await api.addPhotoTags(pid, [tagId]);
        } catch (e) {
          console.warn("添加标签失败:", pid, e);
        }
      }
      for (const pid of photoIds) {
        if (!this.photoTagMap[pid]) this.photoTagMap[pid] = [];
        if (!this.photoTagMap[pid].includes(tagId)) {
          this.photoTagMap[pid].push(tagId);
        }
      }
      await this.loadTags();
    },

    async removeTagFromPhoto(photoId, tagId) {
      await api.removePhotoTag(photoId, tagId);
      if (this.photoTagMap[photoId]) {
        this.photoTagMap[photoId] = this.photoTagMap[photoId].filter((tid) => tid !== tagId);
      }
      await this.loadTags();
    },

    async importPhotos(paths, isFolder) {
      this.importProgress = { done: 0, total: 0 };
      const result = await api.importPhotos(paths, isFolder);
      this.importProgress = null;
      await this.loadPhotos();
      await this.loadStats();
      await this.loadLocationCounts();
      await this.loadAllPhotoTags();
      await this.loadTrips();
      // 导入完成后，后台自动触发逆地理编码（不阻塞）
      this.runBatchGeocode().catch((e) => console.warn("自动逆地理编码失败:", e));
      return result;
    },

    async updateLocation(photoIds, lat, lng, address) {
      await api.updatePhotoLocation({
        photo_ids: photoIds,
        latitude: lat,
        longitude: lng,
        province: null,
        city: null,
        district: null,
        address: address || null,
      });
      await this.loadPhotos();
      await this.loadStats();
      await this.loadLocationCounts();
    },

    async deletePhotos(photoIds) {
      for (const id of photoIds) {
        try {
          await api.deletePhoto(id);
        } catch (e) {
          console.warn("删除失败:", id, e);
        }
      }
      this.selectedPhotoIds = this.selectedPhotoIds.filter(id => !photoIds.includes(id));
      const newMap = { ...this.photoTagMap };
      for (const id of photoIds) delete newMap[id];
      this.photoTagMap = newMap;
      await this.loadPhotos();
      await this.loadStats();
      await this.loadLocationCounts();
      await this.loadTags();
    },

    async runBatchGeocode() {
      if (this.geocodeProgress) return 0;
      this.geocodeProgress = { done: 0, total: 0 };
      try {
        const updated = await api.batchGeocode();
        await this.loadPhotos();
        await this.loadLocationCounts();
        return updated;
      } finally {
        this.geocodeProgress = null;
      }
    },

    async geocodePhoto(photoId) {
      try {
        await api.geocodePhoto(photoId);
        await this.loadPhotos();
        await this.loadLocationCounts();
      } catch (e) {
        console.warn("单张照片逆地理编码失败:", e);
      }
    },

    async addCustomLocation(name, lat, lng, address) {
      await api.addCustomLocation(name, lat, lng, address);
      await this.loadCustomLocations();
    },

    async removeCustomLocation(id) {
      await api.deleteCustomLocation(id);
      await this.loadCustomLocations();
    },

    async setApiKey(key) {
      await api.setConfig("amap_api_key", key);
      this.apiConfigured = !!key;
    },

    async checkApiConfig() {
      const key = await api.getConfig("amap_api_key");
      this.apiConfigured = !!key;
    },

    setFilter(filter) {
      this.filter = { ...this.filter, ...filter };
    },

    resetFilter() {
      this.filter = {
        startDate: null,
        endDate: null,
        province: null,
        city: null,
        lat: null,
        lng: null,
        tagIds: [],
        keyword: "",
      };
    },

    toggleTagFilter(tagId) {
      const idx = (this.filter.tagIds || []).indexOf(tagId);
      if (idx >= 0) {
        this.filter.tagIds.splice(idx, 1);
      } else {
        if (!this.filter.tagIds) this.filter.tagIds = [];
        this.filter.tagIds.push(tagId);
      }
    },

    togglePhotoSelection(id) {
      const idx = this.selectedPhotoIds.indexOf(id);
      if (idx >= 0) {
        this.selectedPhotoIds.splice(idx, 1);
      } else {
        this.selectedPhotoIds.push(id);
      }
    },

    clearSelection() {
      this.selectedPhotoIds = [];
    },
  },
});
