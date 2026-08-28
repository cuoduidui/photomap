<template>
  <div class="dialog-overlay" @click="emit('close')">
    <div class="dialog-card" @click.stop>
      <div class="dialog-header">
        <h3>{{ $t("photoDetail.title") }}</h3>
        <button class="btn-ghost" @click="emit('close')">✕</button>
      </div>

      <div class="dialog-body">
        <div class="detail-image">
          <img v-if="thumbSrc" :src="thumbSrc" alt="" />
          <div v-else class="img-placeholder">{{ $t("photoDetail.noPreview") }}</div>
        </div>

        <div class="detail-info">
          <div class="info-row">
            <span class="info-label">{{ $t("photoDetail.fileName") }}</span>
            <span class="info-value">{{ photo.file_name }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">{{ $t("photoDetail.filePath") }}</span>
            <span class="info-value">{{ photo.file_path }}</span>
          </div>
          <div class="info-row" v-if="photo.file_size">
            <span class="info-label">{{ $t("photoDetail.fileSize") }}</span>
            <span class="info-value">{{ formatSize(photo.file_size) }}</span>
          </div>
          <div class="info-row" v-if="photo.taken_time">
            <span class="info-label">{{ $t("photoDetail.takenAt") }}</span>
            <span class="info-value">{{ photo.taken_time }}</span>
          </div>
          <div class="info-row" v-if="photo.camera_model">
            <span class="info-label">{{ $t("photoDetail.cameraModel") }}</span>
            <span class="info-value">{{ photo.camera_model }}</span>
          </div>
          <div class="info-row" v-if="exifData">
            <span class="info-label">{{ $t("photoDetail.shootingParams") }}</span>
            <span class="info-value">
              <span v-if="exifData.aperture">f/{{ exifData.aperture }} </span>
              <span v-if="exifData.shutter_speed">{{ exifData.shutter_speed }}s </span>
              <span v-if="exifData.iso">ISO{{ exifData.iso }} </span>
              <span v-if="exifData.focal_length">{{ exifData.focal_length }}mm</span>
            </span>
          </div>
          <div class="info-row" v-if="photo.province || photo.city">
            <span class="info-label">{{ $t("photoDetail.geoLocation") }}</span>
            <span class="info-value">
              {{ photo.province || "" }}{{ photo.city ? " · " + photo.city : "" }}{{ photo.district ? " · " + photo.district : "" }}
              <span v-if="geocoding" style="color:var(--muted);font-size:0.75rem;margin-left:6px;">{{ $t("photoDetail.resolving") }}</span>
            </span>
          </div>
          <div class="info-row" v-if="photo.address">
            <span class="info-label">{{ $t("photoDetail.fullAddress") }}</span>
            <span class="info-value">{{ photo.address }}</span>
          </div>
          <div class="info-row" v-if="photo.latitude && !photo.province && geocoding">
            <span class="info-label">{{ $t("photoDetail.locationResolve") }}</span>
            <span class="info-value" style="color:var(--muted);">{{ $t("photoDetail.resolvingLocation") }}</span>
          </div>
          <div class="info-row" v-if="photo.latitude">
            <span class="info-label">{{ $t("photoDetail.latLng") }}</span>
            <span class="info-value">{{ photo.latitude.toFixed(6) }}, {{ photo.longitude.toFixed(6) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">{{ $t("photoDetail.locationSource") }}</span>
            <span class="info-value">
              <span v-if="!photo.latitude" class="tag" style="background:rgba(245,158,11,0.1);color:#d97706;">{{ $t("photoDetail.unlocated") }}</span>
              <span v-else-if="photo.is_location_manual" class="tag" style="background:rgba(59,130,246,0.1);color:#2563eb;">{{ $t("photoDetail.manual") }}</span>
              <span v-else class="tag" style="background:rgba(16,185,129,0.1);color:#10b981;">{{ $t("photoDetail.exifAuto") }}</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { usePhotoStore } from "../stores/photoStore";
import { getImageBase64 } from "../utils/tauri";

const props = defineProps({
  photo: { type: Object, required: true },
});
const emit = defineEmits(["close"]);

const store = usePhotoStore();
const geocoding = ref(false);
const thumbSrc = ref(null);

async function loadThumb() {
  thumbSrc.value = null;
  if (!props.photo.thumbnail_path) return;
  const b64 = await getImageBase64(props.photo.thumbnail_path);
  if (b64) thumbSrc.value = b64;
}

onMounted(() => {
  loadThumb();
});

watch(() => props.photo.id, () => {
  loadThumb();
});

const exifData = computed(() => {
  if (!props.photo.exif_data) return null;
  try {
    return JSON.parse(props.photo.exif_data);
  } catch {
    return null;
  }
});

// 检测是否需要逆地理编码（有坐标但无省市区）
const needsGeocode = computed(() => {
  return props.photo.latitude != null && 
         props.photo.longitude != null && 
         (!props.photo.province || !props.photo.city);
});

// 自动触发单张照片逆地理编码
async function autoGeocode() {
  if (!needsGeocode.value || geocoding.value) return;
  geocoding.value = true;
  try {
    await store.geocodePhoto(props.photo.id);
  } finally {
    geocoding.value = false;
  }
}

onMounted(() => {
  autoGeocode();
});

watch(() => props.photo.id, () => {
  autoGeocode();
});

function formatSize(bytes) {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1048576).toFixed(1) + " MB";
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.dialog-card {
  background: var(--bg2);
  border-radius: 14px;
  width: 560px;
  max-height: 700px;
  overflow-y: auto;
  box-shadow: 0 12px 40px rgba(0,0,0,0.15);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid var(--rule);
}
.dialog-header h3 { font-size: 1.05rem; }

.dialog-body {
  padding: 1rem 1.5rem;
}

.detail-image {
  width: 100%;
  margin-bottom: 1rem;
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg);
}
.detail-image img {
  width: 100%;
  display: block;
}
.img-placeholder {
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  font-size: 0.85rem;
}

.detail-info {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.info-row {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
}
.info-label {
  width: 80px;
  font-size: 0.8rem;
  color: var(--muted);
  flex-shrink: 0;
}
.info-value {
  font-size: 0.85rem;
  color: var(--ink);
  word-break: break-all;
}
</style>
