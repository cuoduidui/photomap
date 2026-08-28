import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export async function importPhotos(paths, isFolder) {
  return invoke("import_photos", { paths, isFolder });
}

export async function cancelLongTask() {
  return invoke("cancel_long_task");
}

export async function getAllPhotos() {
  return invoke("get_all_photos");
}

export async function getPhotoById(id) {
  return invoke("get_photo_by_id", { id });
}

export async function deletePhoto(id) {
  return invoke("delete_photo", { id });
}

export async function getStats() {
  return invoke("get_stats");
}

export async function getLocationCounts() {
  return invoke("get_location_counts");
}

export async function updatePhotoLocation(request) {
  return invoke("update_photo_location", { request });
}

export async function batchGeocode(force = false) {
  return invoke("batch_geocode", { force });
}

export async function geocodePhoto(photoId) {
  return invoke("geocode_photo", { photoId });
}

export async function searchAddress(keyword) {
  return invoke("search_address", { keyword });
}

export async function getCustomLocations() {
  return invoke("get_custom_locations");
}

export async function addCustomLocation(name, latitude, longitude, address) {
  return invoke("add_custom_location", { name, latitude, longitude, address });
}

export async function deleteCustomLocation(id) {
  return invoke("delete_custom_location", { id });
}

// --- Tags ---
export async function getTags() {
  return invoke("get_tags");
}

export async function createTag(name, tagType, color) {
  return invoke("create_tag", { name, tagType, color });
}

export async function deleteTag(id) {
  return invoke("delete_tag", { id });
}

export async function addPhotoTags(photoId, tagIds) {
  return invoke("add_photo_tags", { photoId, tagIds });
}

export async function removePhotoTag(photoId, tagId) {
  return invoke("remove_photo_tag", { photoId, tagId });
}

export async function getPhotoTags(photoId) {
  return invoke("get_photo_tags", { photoId });
}

export async function getAllPhotoTags() {
  return invoke("get_all_photo_tags");
}

export async function analyzeFaces() {
  return invoke("analyze_faces");
}

export async function updateTag(id, name, description, faceThumb) {
  return invoke("update_tag", { id, name, description, faceThumb });
}

export async function getTagPhotos(tagId) {
  return invoke("get_tag_photos", { tagId });
}

export function onFaceAnalyzeProgress(callback) {
  return listen("face-analyze-progress", (event) => {
    const [done, total] = event.payload;
    callback(done, total);
  });
}

export async function debugLog(message) {
  try {
    await invoke("debug_log", { message });
  } catch (e) {}
}

export async function clearDebugLog() {
  try {
    await invoke("clear_debug_log");
  } catch (e) {}
}

export async function saveExportFile(path, dataBase64) {
  return invoke("save_export_file", { path, dataBase64 });
}

export async function exportAlbumImage(albumName, dateRange, location, photoPaths, outputPath) {
  return invoke("export_album_image", { albumName, dateRange, location, photoPaths, outputPath });
}

export async function exportAlbumVideo(albumName, dateRange, photoPaths, outputPath) {
  return invoke("export_album_video", { albumName, dateRange, photoPaths, outputPath });
}

export function onExportProgress(callback) {
  return listen("export-progress", (event) => {
    callback(event.payload[0], event.payload[1]);
  });
}

export async function getConfig(key) {
  return invoke("get_config", { key });
}

export async function setConfig(key, value) {
  return invoke("set_config", { key, value });
}

export async function getMapBounds() {
  return invoke("get_map_bounds");
}

export async function getClusteredPhotos(zoom) {
  return invoke("get_clustered_photos", { zoom });
}

export function onImportProgress(callback) {
  return listen("import-progress", (event) => {
    callback(event.payload[0], event.payload[1]);
  });
}

export function onGeocodeProgress(callback) {
  return listen("geocode-progress", (event) => {
    callback(event.payload[0], event.payload[1]);
  });
}

// 图片缓存（LRU）：限制内存占用，避免长期运行无限增长。
// 缩略图（约 150px）单张 10-50KB，可多缓存一些；原图 base64 可能很大，严格限制数量。
const imageCache = new Map();
const THUMB_CACHE_LIMIT = 400;
const FULL_CACHE_LIMIT = 20;
let thumbCount = 0;
let fullCount = 0;

function cachePut(path, value) {
  const thumb = isThumbnail(path);
  if (imageCache.has(path)) {
    // 刷新 LRU 位置
    imageCache.delete(path);
    if (thumb) thumbCount--; else fullCount--;
  }
  imageCache.set(path, value);
  if (thumb) thumbCount++; else fullCount++;
  // 超出上限时从最久未使用的条目开始淘汰
  const overLimit = thumb ? thumbCount > THUMB_CACHE_LIMIT : fullCount > FULL_CACHE_LIMIT;
  if (overLimit) {
    for (const k of imageCache.keys()) {
      const kThumb = isThumbnail(k);
      if (thumb && kThumb && thumbCount > THUMB_CACHE_LIMIT) {
        imageCache.delete(k);
        thumbCount--;
      } else if (!thumb && !kThumb && fullCount > FULL_CACHE_LIMIT) {
        imageCache.delete(k);
        fullCount--;
      }
      if (!(thumb ? thumbCount > THUMB_CACHE_LIMIT : fullCount > FULL_CACHE_LIMIT)) break;
    }
  }
}

function isThumbnail(path) {
  return /thumb|face_|_thumb/i.test(path);
}

export async function getImageBase64(path) {
  if (!path) return null;
  if (imageCache.has(path)) {
    // LRU：命中时移动到最新位置
    const value = imageCache.get(path);
    imageCache.delete(path);
    imageCache.set(path, value);
    return value;
  }
  try {
    const result = await invoke("get_image_base64", { path });
    cachePut(path, result);
    return result;
  } catch (e) {
    console.warn("加载图片失败:", path, e);
    return null;
  }
}

export async function regenerateThumbnails() {
  return invoke("regenerate_thumbnails");
}

export function clearImageCache() {
  imageCache.clear();
  thumbCount = 0;
  fullCount = 0;
}

// 删除照片后同步清除对应缓存条目，避免残留内存与失效数据
export function removeCachedImages(paths) {
  for (const p of paths) {
    if (p && imageCache.has(p)) {
      imageCache.delete(p);
      if (isThumbnail(p)) thumbCount--; else fullCount--;
    }
  }
}

// --- Trips ---
export async function autoClusterTrips() {
  return invoke("auto_cluster_trips");
}

export async function getAllTrips() {
  return invoke("get_all_trips");
}

export async function getTripPhotos(tripId) {
  return invoke("get_trip_photos", { tripId });
}

export async function saveJournal(tripId, text, journalType) {
  return invoke("save_journal", { tripId, text, journalType });
}

export async function deleteTrip(tripId) {
  return invoke("delete_trip", { tripId });
}

export async function generateAiJournal(tripId) {
  return invoke("generate_ai_journal", { tripId });
}

export async function polishAiJournal(tripId, originalText) {
  return invoke("polish_ai_journal", { tripId, originalText });
}

export function onAiJournalProgress(callback) {
  return listen("ai-journal-progress", (event) => callback(event.payload));
}

export function onAiPromptDebug(callback) {
  return listen("ai-prompt-debug", (event) => callback(event.payload));
}

export async function addPhotosToTrip(tripId, photoIds) {
  return invoke("add_photos_to_trip", { tripId, photoIds });
}

export async function removePhotosFromTrip(tripId, photoIds) {
  return invoke("remove_photos_from_trip", { tripId, photoIds });
}

export async function getPhotosNotInTrip(tripId, page, pageSize) {
  return invoke("get_photos_not_in_trip", { tripId, page, pageSize });
}

export async function createTrip(title, startDate, endDate) {
  return invoke("create_trip", { title, startDate, endDate });
}

export async function generateSlideshow(request) {
  return invoke("generate_slideshow", { request });
}

export function onSlideshowProgress(callback) {
  return listen("slideshow-progress", (event) => {
    callback(event.payload[0], event.payload[1]);
  });
}

export async function openFileLocation(path) {
  return invoke("open_file_location", { path });
}
