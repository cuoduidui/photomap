<template>
  <div class="trip-list">
    <div class="trip-header">
      <span class="trip-title">{{ $t("trip.title") }}</span>
      <div class="header-actions">
        <button class="new-trip-btn" @click="openCreateModal">
          + {{ $t("trip.new") }}
        </button>
        <button class="gen-btn" @click="generateTrips" :disabled="generating">
          {{ generating ? $t('trip.generating') : $t('trip.generateTrips') }}
        </button>
      </div>
    </div>

    <div class="trip-content">
      <div v-if="trips.length === 0 && !generating" class="empty-state">
        <div class="empty-icon">✈️</div>
        <div class="empty-text">{{ $t("trip.noTrips") }}</div>
        <div class="empty-hint">{{ $t("trip.noTripsHint") }}</div>
      </div>

      <div v-else-if="generating" class="loading-state">
        <div class="loading-spinner"></div>
        <div class="loading-text">{{ $t("trip.analyzing") }}</div>
      </div>

      <div v-for="trip in filteredTrips" :key="trip.id" class="trip-card"
        @click="expandTrip(trip)">
        <div class="trip-card-cover">
          <img v-if="coverCache.get(trip.id)" :src="coverCache.get(trip.id)" alt="" />
          <div v-else class="cover-placeholder">
            <span>{{ trip.title.charAt(0) }}</span>
          </div>
          <div class="trip-badge">
            <span>{{ $t("trip.photosCount", { n: trip.photo_count }) }}</span>
          </div>
          <button v-if="trip.is_manual" class="manual-badge" :title="$t('trip.manualBadge')">
            📌
          </button>
        </div>
        <div class="trip-card-info">
          <div class="trip-card-title">{{ trip.title }}</div>
          <div class="trip-card-meta">
            <span class="meta-date">{{ formatDateRange(trip) }}</span>
            <span v-if="trip.cities" class="meta-cities">{{ formatCities(trip) }}</span>
          </div>
          <div class="trip-card-actions" @click.stop>
            <button v-if="!trip.journal_text" class="card-ai-btn"
              @click="quickGenerateJournal(trip)">
              ✨ {{ $t("trip.aiWrite") }}
            </button>
            <button v-else class="card-view-btn"
              @click="expandTrip(trip)">
              📝 {{ $t("trip.viewJournal") }}
            </button>
            <button class="card-delete-btn" @click="deleteTrip(trip)" :title="$t('trip.deleteTrip')">
              🗑️
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 旅行详情弹窗 -->
    <div v-if="activeTrip" class="trip-detail-overlay" @click="activeTrip = null">
      <div class="trip-detail-modal" @click.stop>
        <div class="detail-header">
          <button class="detail-back" @click="activeTrip = null">←</button>
          <div class="detail-title">{{ formatDateRange(activeTrip) }}</div>
        </div>
        <div class="detail-actions">
          <button v-if="!aiGenerating && !showJournalEditor" class="detail-journal-btn ai-btn"
            @click="generateAiJournalText">
            <span>✨ {{ $t("trip.aiGenerateJournal") }}</span>
          </button>
          <button v-if="!aiGenerating" class="detail-journal-btn"
            @click="showJournalEditor = !showJournalEditor">
            {{ activeTrip.journal_text ? $t('trip.editJournal') : $t('trip.writeJournal') }}
          </button>
        </div>

        <div class="detail-meta">
          <span v-if="activeTrip.cities">{{ formatCities(activeTrip) }}</span>
          <span> · {{ $t("trip.photosInTrip", { n: tripPhotos.length }) }}</span>
          <span v-if="activeTrip.journal_type === 'ai_generated'" class="journal-source">{{ $t("trip.aiGenerated") }}</span>
          <span v-else-if="activeTrip.journal_type === 'ai_polished'" class="journal-source polished">{{ $t("trip.aiPolished") }}</span>
          <span v-else-if="activeTrip.journal_text" class="journal-source">{{ $t("trip.handwritten") }}</span>
        </div>

        <!-- AI 生成中 -->
        <div v-if="aiGenerating" class="ai-generating">
          <div class="ai-status">
            <div class="ai-spinner"></div>
            <span>{{ aiStatusText }}</span>
          </div>
          <div class="ai-stream-text">{{ aiStreamText }}<span class="cursor-blink">▌</span></div>
        </div>

        <!-- 游记编辑器 -->
        <div v-else-if="showJournalEditor" class="journal-editor">
          <textarea
            v-model="journalText"
            class="journal-textarea"
            :placeholder="$t('trip.journalPlaceholder')"
            rows="8"
          ></textarea>
          <div class="journal-toolbar">
            <span class="char-count">{{ $t("trip.charCount", { n: journalText.length }) }}</span>
            <div class="toolbar-btns">
              <button class="polish-btn" @click="polishJournalText" :disabled="aiGenerating || !journalText.trim()">
                <span v-if="aiGenerating && polishMode">✨ {{ $t("trip.polishing") }}</span>
                <span v-else>✨ {{ $t("trip.aiPolish") }}</span>
              </button>
              <button class="save-btn" @click="saveJournalText">{{ $t("trip.saveJournal") }}</button>
            </div>
          </div>
          <!-- AI 润色中提示 -->
          <div v-if="aiGenerating && polishMode" class="polish-progress">
            <div class="ai-status-small">
              <div class="ai-spinner-small"></div>
              <span>{{ aiStatusText }}</span>
            </div>
          </div>
        </div>

        <!-- 游记展示 -->
        <div v-else-if="activeTrip.journal_text" class="journal-display">
          <div class="journal-text" v-html="renderJournal(activeTrip.journal_text)"></div>
        </div>

        <!-- 空状态 -->
        <div v-else class="journal-empty">
          <span>{{ $t("trip.journalEmpty") }}</span>
        </div>

        <!-- 照片列表头部 -->
        <div class="photos-header">
          <span class="photos-title">{{ $t("trip.photosTitle", { n: tripPhotos.length }) }}</span>
          <div class="photos-actions">
            <button class="add-photo-btn" @click="openPhotoSelector">
              + {{ $t("trip.addPhotos") }}
            </button>
            <button class="slideshow-btn" @click="openSlideshowModal" :disabled="tripPhotos.length === 0">
              🎬 {{ $t("trip.generateSlideshow") }}
            </button>
          </div>
        </div>

        <!-- 照片列表 -->
        <div class="detail-photos">
          <div v-for="(photo, gi) in pagedTripPhotos" :key="photo.id" class="detail-photo-item"
            :data-idx="(photoPage - 1) * PHOTO_PAGE_SIZE + gi">
            <div class="photo-item-thumb" @click="emit('photo-click', photo)">
              <img v-if="thumbCache.get(photo.id)" :src="thumbCache.get(photo.id)" alt="" />
              <div v-else class="photo-placeholder">
                <span>🖼️</span>
              </div>
            </div>
            <div class="photo-item-info" @click="emit('photo-click', photo)">
              <div class="item-name">{{ photo.file_name }}</div>
              <div class="item-meta">
                <span v-if="photo.taken_time" class="item-date">{{ formatPhotoTime(photo) }}</span>
                <span v-if="photo.city" class="item-city">📍 {{ photo.city }}</span>
              </div>
            </div>
            <button v-if="journalRefs.has((photoPage - 1) * PHOTO_PAGE_SIZE + gi)" class="journal-link-btn"
              :title="$t('trip.locateInJournal', { n: (photoPage - 1) * PHOTO_PAGE_SIZE + gi + 1 })"
              @click.stop="scrollToJournalPhoto((photoPage - 1) * PHOTO_PAGE_SIZE + gi)">
              📖 {{ (photoPage - 1) * PHOTO_PAGE_SIZE + gi + 1 }}
            </button>
            <button class="remove-photo-btn" @click.stop="removePhotoFromTrip(photo)" :title="$t('trip.removeFromTrip')">
              ✕
            </button>
          </div>
        </div>

        <!-- 分页 -->
        <div v-if="totalPhotoPages > 1" class="photo-pagination">
          <button class="page-btn" @click="prevPhotoPage" :disabled="photoPage === 1">
            {{ $t("trip.prevPage") }}
          </button>
          <span class="page-info">
            {{ photoPage }} / {{ totalPhotoPages }}
          </span>
          <button class="page-btn" @click="nextPhotoPage" :disabled="photoPage === totalPhotoPages">
            {{ $t("trip.nextPage") }}
          </button>
        </div>

        <!-- 地图按钮 -->
        <button class="detail-map-btn" @click="showOnMap">
          {{ $t("trip.viewRoute") }}
        </button>
      </div>
    </div>

    <!-- 添加照片选择弹窗 -->
    <div v-if="showPhotoSelector" class="photo-selector-overlay" @click="showPhotoSelector = false">
      <div class="photo-selector-modal" @click.stop>
        <div class="selector-header">
          <span class="selector-title">{{ $t("trip.selectorTitle") }}</span>
          <button class="selector-close" @click="showPhotoSelector = false">✕</button>
        </div>

        <div class="selector-stats">
          {{ $t("trip.selectedCount", { n: selectedPhotoIds.size, m: availableTotal }) }}
        </div>

        <div class="selector-photos">
          <div v-for="photo in pagedAvailablePhotos" :key="photo.id" 
            class="selector-photo-item"
            :class="{ selected: selectedPhotoIds.has(photo.id) }"
            @click="togglePhotoSelection(photo)">
            <div class="selector-photo-thumb">
              <img v-if="selectorThumbCache.get(photo.id)" :src="selectorThumbCache.get(photo.id)" alt="" />
              <div v-else class="selector-placeholder">🖼️</div>
            </div>
            <div class="selector-photo-info">
              <div class="selector-photo-name">{{ photo.file_name }}</div>
              <div class="selector-photo-meta">
                <span v-if="photo.taken_time" class="selector-date">{{ formatSelectorPhotoTime(photo) }}</span>
                <span v-if="photo.city" class="selector-city">📍 {{ photo.city }}</span>
              </div>
            </div>
            <div class="selector-check">
              <span v-if="selectedPhotoIds.has(photo.id)">✓</span>
            </div>
          </div>
          <div v-if="availableLoading" class="selector-loading">
            <div class="loading-spinner"></div>
            <span>{{ $t("trip.loading") }}</span>
          </div>
        </div>

        <!-- 分页 -->
        <div v-if="availableTotalPages > 1" class="selector-pagination">
          <button class="page-btn" @click="prevAvailablePage" :disabled="availablePage === 1">
            {{ $t("trip.prevPage") }}
          </button>
          <span class="page-info">
            {{ availablePage }} / {{ availableTotalPages }}
          </span>
          <button class="page-btn" @click="nextAvailablePage" :disabled="availablePage === availableTotalPages">
            {{ $t("trip.nextPage") }}
          </button>
        </div>

        <div class="selector-footer">
          <button class="select-all-btn" @click="toggleSelectAll">
            {{ isAllSelected ? $t('trip.clearAll') : $t('trip.selectAll') }}
          </button>
          <div class="footer-actions">
            <button class="cancel-btn" @click="showPhotoSelector = false">{{ $t("common.cancel") }}</button>
            <button class="confirm-btn" 
              @click="confirmAddPhotos" 
              :disabled="selectedPhotoIds.size === 0 || addingPhotos">
              {{ addingPhotos ? $t('trip.adding') : $t('trip.addNPhotos', { n: selectedPhotoIds.size }) }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 新建游记弹窗 -->
    <div v-if="showCreateModal" class="create-modal-overlay" @click="showCreateModal = false">
      <div class="create-modal" @click.stop>
        <div class="create-header">
          <span class="create-title">{{ $t("trip.createTitle") }}</span>
          <button class="create-close" @click="showCreateModal = false">✕</button>
        </div>
        <div class="create-body">
          <div class="form-item">
            <label class="form-label">{{ $t("trip.tripTitleLabel") }}</label>
            <input v-model="newTripTitle" class="form-input" 
              :placeholder="$t('trip.titlePlaceholder')" maxlength="50" />
          </div>
          <div class="form-row">
            <div class="form-item">
              <label class="form-label">{{ $t("trip.startDate") }}</label>
              <input v-model="newTripStart" type="date" class="form-input" />
            </div>
            <div class="form-item">
              <label class="form-label">{{ $t("trip.endDate") }}</label>
              <input v-model="newTripEnd" type="date" class="form-input" />
            </div>
          </div>
          <p class="form-hint">
            💡 {{ $t("trip.createHint") }}
          </p>
        </div>
        <div class="create-footer">
          <button class="cancel-btn" @click="showCreateModal = false">{{ $t("common.cancel") }}</button>
          <button class="confirm-btn" @click="confirmCreateTrip" 
            :disabled="!newTripTitle.trim() || creatingTrip">
            {{ creatingTrip ? $t('trip.creating') : $t('trip.createTrip') }}
          </button>
        </div>
      </div>
    </div>

    <!-- AI 影集生成弹窗 -->
    <div v-if="showSlideshowModal" class="slideshow-modal-overlay" @click="showSlideshowModal = false">
      <div class="slideshow-modal" @click.stop>
        <div class="slideshow-header">
          <span class="slideshow-title">🎬 {{ $t("trip.slideshowTitle") }}</span>
          <button class="slideshow-close" @click="closeSlideshowModal">✕</button>
        </div>

        <div v-if="!slideshowGenerating && !slideshowResult" class="slideshow-body">
          <!-- 影集标题 -->
          <div class="form-item">
            <label class="form-label">{{ $t("trip.slideshowTitleLabel") }}</label>
            <input v-model="slideshowTitle" class="form-input" 
              :placeholder="$t('trip.slideshowTitlePlaceholder')" maxlength="50" />
          </div>

          <!-- 旁白文案 -->
          <div class="form-item">
            <div class="form-label-row">
              <label class="form-label">{{ $t("trip.narrationLabel") }}</label>
              <button class="ai-gen-narration-btn" @click="aiGenerateNarration" :disabled="aiNarrationGenerating || tripPhotos.length === 0">
                <span v-if="aiNarrationGenerating">{{ $t("trip.generating") }}</span>
                <span v-else>✨ {{ $t("trip.aiGenerateNarration") }}</span>
              </button>
            </div>
            <textarea
              v-model="slideshowNarration"
              class="narration-textarea"
              :placeholder="$t('trip.narrationPlaceholder')"
              rows="5"
            ></textarea>
            <div class="narration-hint">
              {{ $t("trip.narrationHint", { n: slideshowNarration.length }) }}
            </div>
          </div>

          <!-- 参数配置 -->
          <div class="form-item">
            <label class="form-label">{{ $t("trip.videoParams") }}</label>
            <div class="params-grid">
              <div class="param-item">
                <span class="param-label">{{ $t("trip.resolution") }}</span>
                <select v-model="slideshowResolution" class="param-select">
                  <option value="1080p">1080p (1920×1080)</option>
                  <option value="720p">720p (1280×720)</option>
                  <option value="4k">4K (3840×2160)</option>
                </select>
              </div>
              <div class="param-item">
                <span class="param-label">{{ $t("trip.photoDuration") }}</span>
                <select v-model="slideshowPhotoDuration" class="param-select">
                  <option :value="2">{{ $t("trip.seconds", { n: 2 }) }}</option>
                  <option :value="3">{{ $t("trip.seconds", { n: 3 }) }}</option>
                  <option :value="4">{{ $t("trip.seconds", { n: 4 }) }}</option>
                  <option :value="5">{{ $t("trip.seconds", { n: 5 }) }}</option>
                </select>
              </div>
              <div class="param-item">
                <span class="param-label">{{ $t("trip.transitionDuration") }}</span>
                <select v-model="slideshowTransition" class="param-select">
                  <option :value="0.5">{{ $t("trip.seconds", { n: 0.5 }) }}</option>
                  <option :value="1">{{ $t("trip.seconds", { n: 1 }) }}</option>
                  <option :value="1.5">{{ $t("trip.seconds", { n: 1.5 }) }}</option>
                  <option :value="2">{{ $t("trip.seconds", { n: 2 }) }}</option>
                </select>
              </div>
              <div class="param-item">
                <span class="param-label">{{ $t("trip.style") }}</span>
                <select v-model="slideshowStyle" class="param-select">
                  <option value="realistic">{{ $t("trip.styleRealistic") }}</option>
                  <option value="cinematic">{{ $t("trip.styleCinematic") }}</option>
                  <option value="warm">{{ $t("trip.styleWarm") }}</option>
                </select>
              </div>
            </div>
          </div>

          <!-- 背景音乐 -->
          <div class="form-item">
            <label class="form-label">{{ $t("trip.bgMusic") }}</label>
            <div class="music-selector">
              <input v-model="slideshowMusicPath" class="form-input music-input" 
                :placeholder="$t('trip.musicPlaceholder')" readonly />
              <button class="select-music-btn" @click="selectMusicFile">
                {{ $t("trip.chooseFile") }}
              </button>
            </div>
            <div v-if="slideshowMusicPath" class="music-clear">
              <button @click="slideshowMusicPath = ''">{{ $t("trip.clearMusic") }}</button>
            </div>
          </div>

          <div class="slideshow-info">
            <span>📷 {{ $t("trip.photosTotal", { n: tripPhotos.length }) }}</span>
            <span>⏱️ {{ $t("trip.estimatedDuration", { n: estimatedDuration }) }}</span>
          </div>
        </div>

        <!-- 生成中 -->
        <div v-if="slideshowGenerating" class="slideshow-generating">
          <div class="generating-icon">🎬</div>
          <div class="generating-title">{{ $t("trip.generatingSlideshow") }}</div>
          <div class="generating-progress-bar">
            <div class="progress-fill" :style="{ width: slideshowProgress + '%' }"></div>
          </div>
          <div class="generating-progress-text">{{ slideshowProgress }}%</div>
          <div class="generating-hint">
            {{ generatingStageText }}
          </div>
        </div>

        <!-- 生成结果 -->
        <div v-if="slideshowResult" class="slideshow-result">
          <div class="result-icon">✅</div>
          <div class="result-title">{{ $t("trip.slideshowDone") }}</div>
          <div class="result-path">{{ slideshowResult }}</div>
          <div class="result-actions">
            <button class="result-btn primary" @click="openSlideshowFolder">
              📂 {{ $t("trip.openFolder") }}
            </button>
            <button class="result-btn" @click="resetSlideshow">
              {{ $t("trip.regenerate") }}
            </button>
          </div>
        </div>

        <div v-if="!slideshowGenerating && !slideshowResult" class="slideshow-footer">
          <button class="cancel-btn" @click="showSlideshowModal = false">{{ $t("common.cancel") }}</button>
          <button class="confirm-btn" @click="startGenerateSlideshow" 
            :disabled="tripPhotos.length === 0 || slideshowGenerating">
            {{ slideshowGenerating ? $t('trip.generating') : $t('trip.startGenerate') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 照片查看器 -->
    <PhotoViewer
      v-if="viewerVisible"
      :photo="tripPhotos[viewerIndex]"
      :photo-list="tripPhotos"
      @close="viewerVisible = false" />
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { getTripPhotos, saveJournal, getImageBase64, generateAiJournal, polishAiJournal, onAiJournalProgress, addPhotosToTrip, removePhotosFromTrip, getPhotosNotInTrip, createTrip, deleteTrip as deleteTripApi, generateSlideshow, onSlideshowProgress, openFileLocation } from "../utils/tauri";
import { translateProgressText } from "../i18n/backendErrors";
import { getLocale } from "../i18n";
import PhotoViewer from "./PhotoViewer.vue";

const emit = defineEmits(["photo-click", "focus-location"]);
const store = usePhotoStore();
const { t } = useI18n();

const trips = ref([]);
const generating = ref(false);
const activeTrip = ref(null);
const tripPhotos = ref([]);
const showJournalEditor = ref(false);
const journalText = ref("");
const coverCache = ref(new Map());
const thumbCache = ref(new Map());

// 照片分页
const photoPage = ref(1);
const PHOTO_PAGE_SIZE = 8;

const totalPhotoPages = computed(() => {
  return Math.ceil(tripPhotos.value.length / PHOTO_PAGE_SIZE);
});

const pagedTripPhotos = computed(() => {
  const start = (photoPage.value - 1) * PHOTO_PAGE_SIZE;
  const end = start + PHOTO_PAGE_SIZE;
  return tripPhotos.value.slice(start, end);
});

// 游记正文中引用到的照片索引（[photo:N] → N-1）
const journalRefs = computed(() => {
  const set = new Set();
  const text = (activeTrip.value && activeTrip.value.journal_text) || "";
  const re = /\[photo:(\d+)\]/gi;
  let m;
  while ((m = re.exec(text))) {
    const idx = parseInt(m[1], 10) - 1;
    if (idx >= 0 && idx < tripPhotos.value.length) set.add(idx);
  }
  return set;
});

function prevPhotoPage() {
  if (photoPage.value > 1) {
    photoPage.value--;
    loadCurrentPageThumbs();
  }
}

function nextPhotoPage() {
  if (photoPage.value < totalPhotoPages.value) {
    photoPage.value++;
    loadCurrentPageThumbs();
  }
}

async function loadCurrentPageThumbs() {
  const photos = pagedTripPhotos.value;
  for (const p of photos) {
    if (p.thumbnail_path && !thumbCache.value.has(p.id)) {
      try {
        const b64 = await getImageBase64(p.thumbnail_path);
        if (b64) {
          thumbCache.value.set(p.id, b64);
          thumbCache.value = new Map(thumbCache.value);
        }
      } catch {}
    }
  }
}

const aiGenerating = ref(false);
const aiStatusText = ref("");
const aiStreamText = ref("");
const polishMode = ref(false);
let unlistenAiProgress = null;

// 照片选择器
const showPhotoSelector = ref(false);
const availablePhotos = ref([]);
const availableTotal = ref(0);
const availablePage = ref(1);
const availableLoading = ref(false);
const addingPhotos = ref(false);
const selectedPhotoIds = ref(new Set());
const selectorThumbCache = ref(new Map());
const AVAILABLE_PAGE_SIZE = 10;

const availableTotalPages = computed(() => {
  return Math.ceil(availableTotal.value / AVAILABLE_PAGE_SIZE);
});

const pagedAvailablePhotos = computed(() => {
  return availablePhotos.value;
});

const isAllSelected = computed(() => {
  return availablePhotos.value.every(p => selectedPhotoIds.value.has(p.id));
});

// 过滤：自动生成的游记没有照片则不展示，手动创建的游记始终展示
const filteredTrips = computed(() => {
  return trips.value.filter(t => t.is_manual || t.photo_count > 0);
});

// 新建游记
const showCreateModal = ref(false);
const newTripTitle = ref("");
const newTripStart = ref("");
const newTripEnd = ref("");
const creatingTrip = ref(false);

onMounted(async () => {
  trips.value = store.trips || [];
  loadCovers();
  unlistenAiProgress = await onAiJournalProgress((data) => {
    if (data.type === "status") {
      aiStatusText.value = translateProgressText(data.text);
    } else if (data.type === "chunk") {
      aiStreamText.value += data.text;
    } else if (data.type === "done") {
      aiStreamText.value = data.text;
    }
  });
});

onUnmounted(() => {
  if (unlistenAiProgress) unlistenAiProgress();
});

watch(() => store.trips, (newTrips) => {
  trips.value = newTrips || [];
  loadCovers();
}, { immediate: true });

onMounted(() => {
  trips.value = store.trips || [];
  loadCovers();
});

async function loadCovers() {
  for (const trip of trips.value) {
    if (coverCache.value.has(trip.id)) continue;
    const photos = await getTripPhotos(trip.id);
    const coverPhoto = photos.find(p => p.thumbnail_path) || photos[0];
    if (coverPhoto && coverPhoto.thumbnail_path) {
      const b64 = await getImageBase64(coverPhoto.thumbnail_path);
      if (b64) {
        coverCache.value.set(trip.id, b64);
        coverCache.value = new Map(coverCache.value);
      }
    }
  }
}

async function generateTrips() {
  generating.value = true;
  try {
    await store.generateTrips();
    trips.value = store.trips;
    loadCovers();
  } catch (e) {
    console.error("生成旅行失败:", e);
  } finally {
    generating.value = false;
  }
}

async function expandTrip(trip) {
  activeTrip.value = trip;
  showJournalEditor.value = false;
  aiGenerating.value = false;
  aiStreamText.value = "";
  aiStatusText.value = "";
  journalText.value = trip.journal_text || "";
  photoPage.value = 1;
  tripPhotos.value = await getTripPhotos(trip.id);
  // 同步更新列表中的照片数
  if (trip.photo_count !== tripPhotos.value.length) {
    trip.photo_count = tripPhotos.value.length;
    const t = trips.value.find(t => t.id === trip.id);
    if (t) t.photo_count = tripPhotos.value.length;
  }
  loadCurrentPageThumbs();
  // 加载游记中的图片
  if (trip.journal_text && trip.journal_text.includes("[photo:")) {
    loadJournalPhotos();
  }
}

async function generateAiJournalText() {
  if (!activeTrip.value || aiGenerating.value) return;
  aiGenerating.value = true;
  aiStreamText.value = "";
  aiStatusText.value = t("trip.statusPreparing");
  try {
    const result = await generateAiJournal(activeTrip.value.id, getLocale());
    activeTrip.value.journal_text = result;
    activeTrip.value.journal_type = "ai_generated";
    journalText.value = result;
    await store.loadTrips();
    // 加载图文游记中的图片
    if (result.includes("[photo:")) {
      await nextTick();
      loadJournalPhotos();
    }
  } catch (e) {
    console.error("AI 生成失败:", e);
    aiStatusText.value = t("trip.aiGenerateFailed", { error: e });
  } finally {
    aiGenerating.value = false;
  }
}

async function quickGenerateJournal(trip) {
  await expandTrip(trip);
  generateAiJournalText();
}

async function polishJournalText() {
  if (!activeTrip.value || aiGenerating.value) return;
  if (!journalText.value.trim()) return;

  polishMode.value = true;
  aiGenerating.value = true;
  aiStreamText.value = "";
  aiStatusText.value = t("trip.statusPreparing");

  try {
    const result = await polishAiJournal(activeTrip.value.id, journalText.value, getLocale());
    journalText.value = result;
    activeTrip.value.journal_text = result;
    activeTrip.value.journal_type = "ai_polished";
    await store.loadTrips();
  } catch (e) {
    console.error("AI 润色失败:", e);
    aiStatusText.value = t("trip.polishFailed", { error: e });
  } finally {
    aiGenerating.value = false;
    polishMode.value = false;
  }
}

async function loadThumbs() {
  for (const photo of tripPhotos.value) {
    if (thumbCache.value.has(photo.id)) continue;
    if (photo.thumbnail_path) {
      const b64 = await getImageBase64(photo.thumbnail_path);
      if (b64) {
        thumbCache.value.set(photo.id, b64);
        thumbCache.value = new Map(thumbCache.value);
      }
    }
  }
}

async function saveJournalText() {
  if (!activeTrip.value) return;
  try {
    await saveJournal(activeTrip.value.id, journalText.value, journalText.value ? "manual" : null);
    activeTrip.value.journal_text = journalText.value;
    activeTrip.value.journal_type = journalText.value ? "manual" : null;
    showJournalEditor.value = false;
    await store.loadTrips();
  } catch (e) {
    console.error("保存游记失败:", e);
  }
}

function showOnMap() {
  if (tripPhotos.value.length > 0) {
    const photo = tripPhotos.value[0];
    if (photo.latitude && photo.longitude) {
      emit("focus-location", { lat: photo.latitude, lng: photo.longitude });
    }
  }
  activeTrip.value = null;
}

function formatDateRange(trip) {
  if (!trip.start_date || !trip.end_date) return "";
  const start = trip.start_date.replace(/-/g, "/");
  const end = trip.end_date.replace(/-/g, "/");
  return `${start} - ${end}`;
}

function formatCities(trip) {
  if (!trip.city_names) return "";
  try {
    const cities = JSON.parse(trip.city_names);
    return Array.isArray(cities) ? cities.join(" · ") : cities;
  } catch {
    return trip.city_names;
  }
}

function formatPhotoDate(photo) {
  if (!photo.taken_time) return "";
  const d = new Date(photo.taken_time);
  if (isNaN(d.getTime())) return "";
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

function formatPhotoTime(photo) {
  if (!photo.taken_time) return "";
  const d = new Date(photo.taken_time);
  if (isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

function renderJournal(text) {
  if (!text) return "";
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // 把 [photo:N] 替换为图片占位符，用 span 标记，后续用 JS 填充
  let html = escaped.replace(/\[photo:(\d+)\]/gi, (_, num) => {
    const idx = parseInt(num) - 1;
    return `<span class="journal-photo-placeholder" data-idx="${idx}">📷 ${t("trip.photoPlaceholder", { n: num })}</span>`;
  });

  return html.replace(/\n/g, "<br>");
}

// 加载游记中的图片
async function loadJournalPhotos() {
  if (!activeTrip.value || !activeTrip.value.journal_text) return;

  await nextTick();

  const journalEl = document.querySelector(".journal-text");
  if (!journalEl) return;

  const placeholders = journalEl.querySelectorAll(".journal-photo-placeholder");
  for (const el of placeholders) {
    const idx = parseInt(el.dataset.idx);
    const photo = tripPhotos.value[idx];
    if (photo && photo.thumbnail_path) {
      try {
        const b64 = await getImageBase64(photo.thumbnail_path);
        if (b64) {
          const div = document.createElement("div");
          div.className = "journal-photo";
          div.dataset.idx = idx;
          div.innerHTML = `
            <img src="${b64}" alt="${t("trip.altPhoto", { n: idx + 1 })}" />
            <div class="journal-photo-caption">
              <span class="journal-photo-num">📷 ${idx + 1} / ${tripPhotos.value.length}</span>
              <span class="journal-locate-btn" title="${t("trip.locateToPhotoList")}">⬇ ${t("trip.locateToPhotoList")}</span>
            </div>
          `;
          div.addEventListener("click", () => {
            viewerIndex.value = idx;
            viewerVisible.value = true;
          });
          const locateBtn = div.querySelector(".journal-locate-btn");
          if (locateBtn) {
            locateBtn.addEventListener("click", (e) => {
              e.stopPropagation();
              scrollToPhotoGrid(idx);
            });
          }
          el.replaceWith(div);
        }
      } catch {}
    }
  }
}

// 高亮闪烁辅助
function flashElement(el, cls) {
  el.classList.remove(cls);
  void el.offsetWidth;
  el.classList.add(cls);
  setTimeout(() => el.classList.remove(cls), 1800);
}

// 从游记定位到照片网格
function scrollToPhotoGrid(idx) {
  const targetPage = Math.floor(idx / PHOTO_PAGE_SIZE) + 1;
  if (photoPage.value !== targetPage) {
    photoPage.value = targetPage;
    loadCurrentPageThumbs();
  }
  nextTick(() => {
    const el = document.querySelector(`.detail-photo-item[data-idx="${idx}"]`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      flashElement(el, "flash");
    }
  });
}

// 从照片网格定位到游记正文中的对应插图
function scrollToJournalPhoto(idx) {
  const journalEl = document.querySelector(".journal-text");
  if (!journalEl) return;
  const find = () => journalEl.querySelector(`.journal-photo[data-idx="${idx}"]`);
  const el = find();
  if (!el) {
    // 插图尚未加载完成时，先补载再定位
    loadJournalPhotos().then(() => {
      nextTick(() => {
        const el2 = find();
        if (el2) {
          el2.scrollIntoView({ behavior: "smooth", block: "center" });
          flashElement(el2, "flash");
        }
      });
    });
    return;
  }
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    flashElement(el, "flash");
  }
}

// 打开照片选择器
async function openPhotoSelector() {
  if (!activeTrip.value) return;
  showPhotoSelector.value = true;
  selectedPhotoIds.value = new Set();
  availablePage.value = 1;
  await loadAvailablePhotos();
}

// 加载可选照片
async function loadAvailablePhotos() {
  if (!activeTrip.value) return;
  availableLoading.value = true;
  try {
    const [photos, total] = await getPhotosNotInTrip(
      activeTrip.value.id,
      availablePage.value,
      AVAILABLE_PAGE_SIZE
    );
    availablePhotos.value = photos;
    availableTotal.value = total;
    loadSelectorThumbs(photos);
  } catch (e) {
    console.error("加载可选照片失败:", e);
  } finally {
    availableLoading.value = false;
  }
}

// 加载选择器缩略图
async function loadSelectorThumbs(photos) {
  for (const p of photos) {
    if (p.thumbnail_path && !selectorThumbCache.value.has(p.id)) {
      try {
        const b64 = await getImageBase64(p.thumbnail_path);
        if (b64) {
          selectorThumbCache.value.set(p.id, b64);
          selectorThumbCache.value = new Map(selectorThumbCache.value);
        }
      } catch {}
    }
  }
}

// 切换照片选择
function togglePhotoSelection(photo) {
  const newSet = new Set(selectedPhotoIds.value);
  if (newSet.has(photo.id)) {
    newSet.delete(photo.id);
  } else {
    newSet.add(photo.id);
  }
  selectedPhotoIds.value = newSet;
}

// 全选/取消全选当前页
function toggleSelectAll() {
  const newSet = new Set(selectedPhotoIds.value);
  if (isAllSelected.value) {
    // 取消全选当前页
    for (const p of availablePhotos.value) {
      newSet.delete(p.id);
    }
  } else {
    // 全选当前页
    for (const p of availablePhotos.value) {
      newSet.add(p.id);
    }
  }
  selectedPhotoIds.value = newSet;
}

// 上一页
async function prevAvailablePage() {
  if (availablePage.value > 1) {
    availablePage.value--;
    await loadAvailablePhotos();
  }
}

// 下一页
async function nextAvailablePage() {
  if (availablePage.value < availableTotalPages.value) {
    availablePage.value++;
    await loadAvailablePhotos();
  }
}

// 确认添加照片
async function confirmAddPhotos() {
  if (!activeTrip.value || selectedPhotoIds.value.size === 0) return;
  addingPhotos.value = true;
  try {
    const ids = Array.from(selectedPhotoIds.value);
    await addPhotosToTrip(activeTrip.value.id, ids);
    // 刷新游记照片列表
    tripPhotos.value = await getTripPhotos(activeTrip.value.id);
    photoPage.value = 1;
    loadCurrentPageThumbs();
    // 更新游记照片数
    activeTrip.value.photo_count = tripPhotos.value.length;
    // 刷新 store
    await store.loadTrips();
    trips.value = store.trips;
    loadCovers();
    // 关闭选择器
    showPhotoSelector.value = false;
  } catch (e) {
    console.error("添加照片失败:", e);
    alert(t("trip.addFailed", { error: e }));
  } finally {
    addingPhotos.value = false;
  }
}

// 从游记移除照片
async function removePhotoFromTrip(photo) {
  if (!activeTrip.value) return;
  if (!confirm(t("trip.removePhotoConfirm", { name: photo.file_name }))) return;
  try {
    await removePhotosFromTrip(activeTrip.value.id, [photo.id]);
    // 刷新游记照片列表
    tripPhotos.value = await getTripPhotos(activeTrip.value.id);
    // 如果当前页没有照片了且不是第一页，回到上一页
    if (pagedTripPhotos.value.length === 0 && photoPage.value > 1) {
      photoPage.value--;
    }
    loadCurrentPageThumbs();
    // 更新游记照片数
    activeTrip.value.photo_count = tripPhotos.value.length;
    // 刷新 store
    await store.loadTrips();
    trips.value = store.trips;
    loadCovers();
  } catch (e) {
    console.error("移除照片失败:", e);
    alert(t("trip.removeFailed", { error: e }));
  }
}

function formatSelectorPhotoTime(photo) {
  if (!photo.taken_time) return "";
  const d = new Date(photo.taken_time);
  if (isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

// 打开新建游记弹窗
function openCreateModal() {
  newTripTitle.value = "";
  newTripStart.value = "";
  newTripEnd.value = "";
  showCreateModal.value = true;
}

// 确认创建游记
async function confirmCreateTrip() {
  if (!newTripTitle.value.trim() || creatingTrip.value) return;
  creatingTrip.value = true;
  try {
    const newTrip = await createTrip(
      newTripTitle.value.trim(),
      newTripStart.value || null,
      newTripEnd.value || null
    );
    // 标记为手动创建
    newTrip.is_manual = true;
    await store.loadTrips();
    trips.value = store.trips;
    loadCovers();
    showCreateModal.value = false;
    // 自动打开新建的游记
    expandTrip(newTrip);
  } catch (e) {
    console.error("创建游记失败:", e);
    alert(t("trip.createFailed", { error: e }));
  } finally {
    creatingTrip.value = false;
  }
}

// 删除游记
async function deleteTrip(trip) {
  if (!confirm(t("trip.deleteConfirm", { title: trip.title }))) return;
  try {
    await deleteTripApi(trip.id);
    // 从列表中移除
    trips.value = trips.value.filter(t => t.id !== trip.id);
    coverCache.value.delete(trip.id);
    // 如果当前打开的是这个游记，关闭详情
    if (activeTrip.value && activeTrip.value.id === trip.id) {
      activeTrip.value = null;
    }
    await store.loadTrips();
  } catch (e) {
    console.error("删除游记失败:", e);
    alert(t("trip.deleteFailed", { error: e }));
  }
}

// ==================== AI 影集生成 ====================
const showSlideshowModal = ref(false);
const viewerVisible = ref(false);
const viewerIndex = ref(0);
const slideshowTitle = ref("");
const slideshowNarration = ref("");
const slideshowResolution = ref("1080p");
const slideshowPhotoDuration = ref(3);
const slideshowTransition = ref(1);
const slideshowStyle = ref("realistic");
const slideshowMusicPath = ref("");
const slideshowGenerating = ref(false);
const slideshowProgress = ref(0);
const slideshowResult = ref("");
const aiNarrationGenerating = ref(false);
let unlistenSlideshowProgress = null;

const resolutionMap = {
  "720p": { width: 1280, height: 720 },
  "1080p": { width: 1920, height: 1080 },
  "4k": { width: 3840, height: 2160 },
};

const estimatedDuration = computed(() => {
  const count = tripPhotos.value.length;
  if (count === 0) return 0;
  return Math.round(count * (slideshowPhotoDuration.value + slideshowTransition.value));
});

const generatingStageText = computed(() => {
  const p = slideshowProgress.value;
  if (p < 5) return t("trip.statusPreparing");
  if (p < 35) return t("trip.statusProcessing");
  if (p < 45) return t("trip.statusPreparingVideo");
  if (p < 80) return t("trip.statusComposing");
  if (p < 100) return t("trip.statusAddingMusic");
  return t("trip.statusDone");
});

function openSlideshowModal() {
  if (!activeTrip.value || tripPhotos.value.length === 0) return;
  slideshowTitle.value = activeTrip.value.title || "";
  slideshowNarration.value = activeTrip.value.journal_text || "";
  slideshowProgress.value = 0;
  slideshowResult.value = "";
  slideshowGenerating.value = false;
  showSlideshowModal.value = true;
}

function closeSlideshowModal() {
  if (slideshowGenerating.value) {
    if (!confirm(t("trip.closeConfirm"))) return;
  }
  showSlideshowModal.value = false;
}

async function selectMusicFile() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      filters: [
        { name: t("trip.audioFiles"), extensions: ["mp3", "wav", "m4a", "flac", "aac"] }
      ]
    });
    if (selected) {
      slideshowMusicPath.value = selected;
    }
  } catch (e) {
    console.error("选择音乐文件失败:", e);
    alert(t("trip.selectMusicFailed", { error: e }));
  }
}

async function aiGenerateNarration() {
  if (!activeTrip.value || aiNarrationGenerating.value) return;
  aiNarrationGenerating.value = true;
  try {
    // 使用现有的 AI 游记生成功能来生成旁白
    const result = await generateAiJournal(activeTrip.value.id, getLocale());
    // 精简一下作为旁白
    const sentences = result.split(/[。！？.!?]/).filter(s => s.trim().length > 0);
    const narration = sentences.slice(0, Math.min(sentences.length, 8)).join("。") + "。";
    slideshowNarration.value = narration;
  } catch (e) {
    console.error("AI 生成旁白失败:", e);
    alert(t("trip.aiNarrationFailed", { error: e }));
  } finally {
    aiNarrationGenerating.value = false;
  }
}

async function startGenerateSlideshow() {
  if (!activeTrip.value || tripPhotos.value.length === 0) return;
  if (slideshowGenerating.value) return;

  slideshowGenerating.value = true;
  slideshowProgress.value = 0;
  slideshowResult.value = "";

  try {
    // 监听进度
    if (unlistenSlideshowProgress) unlistenSlideshowProgress();
    unlistenSlideshowProgress = await onSlideshowProgress((progress, total) => {
      slideshowProgress.value = progress;
    });

    // 准备照片路径
    const photoPaths = tripPhotos.value.map(p => p.file_path).filter(Boolean);

    // 获取分辨率
    const res = resolutionMap[slideshowResolution.value] || resolutionMap["1080p"];

    // 生成输出路径（使用保存对话框）
    const { save } = await import("@tauri-apps/plugin-dialog");
    const defaultName = `${slideshowTitle.value || t('trip.slideshowDefaultName')}_${Date.now()}.mp4`;
    const outputPath = await save({
      defaultPath: defaultName,
      filters: [
        { name: t("trip.mp4Video"), extensions: ["mp4"] }
      ]
    });

    if (!outputPath) {
      slideshowGenerating.value = false;
      return;
    }

    const request = {
      trip_id: activeTrip.value.id,
      title: slideshowTitle.value,
      narration: slideshowNarration.value,
      photo_paths: photoPaths,
      output_path: outputPath,
      photo_duration: slideshowPhotoDuration.value,
      transition_duration: slideshowTransition.value,
      width: res.width,
      height: res.height,
      music_path: slideshowMusicPath.value || null,
    };

    const result = await generateSlideshow(request);
    slideshowResult.value = result;
  } catch (e) {
    console.error("生成影集失败:", e);
    alert(t("trip.slideshowFailed", { error: e }));
    slideshowGenerating.value = false;
  } finally {
    slideshowGenerating.value = false;
    if (unlistenSlideshowProgress) {
      unlistenSlideshowProgress();
      unlistenSlideshowProgress = null;
    }
  }
}

function resetSlideshow() {
  slideshowResult.value = "";
  slideshowProgress.value = 0;
}

async function openSlideshowFolder() {
  if (!slideshowResult.value) return;
  try {
    await openFileLocation(slideshowResult.value);
  } catch (e) {
    console.error("打开文件夹失败:", e);
  }
}

onUnmounted(() => {
  if (unlistenSlideshowProgress) unlistenSlideshowProgress();
});
</script>

<style scoped>
.trip-list {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.trip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
}
.trip-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}
.gen-btn {
  font-size: 0.7rem;
  color: #fff;
  padding: 4px 10px;
  border-radius: 10px;
  background: var(--accent, #10ac84);
  box-shadow: 0 2px 8px rgba(16, 172, 132, 0.2);
}
.gen-btn:hover:not(:disabled) {
  transform: translateY(-1px);
}
.gen-btn:disabled {
  opacity: 0.5;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.new-trip-btn {
  font-size: 0.7rem;
  color: var(--accent);
  padding: 4px 10px;
  border-radius: 10px;
  background: rgba(99, 102, 241, 0.1);
  font-weight: 500;
  transition: all 0.15s ease;
}
.new-trip-btn:hover {
  background: rgba(99, 102, 241, 0.2);
}

.trip-content {
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 12px;
}

.trip-card {
  display: flex;
  gap: 12px;
  padding: 10px;
  margin-bottom: 10px;
  border-radius: var(--radius);
  background: var(--bg-card);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: all 0.2s ease;
}
.trip-card:hover {
  border-color: var(--border-glow);
  box-shadow: var(--shadow-md);
  transform: translateY(-1px);
}

.trip-card-cover {
  width: 72px;
  height: 72px;
  border-radius: var(--radius-sm);
  overflow: hidden;
  flex-shrink: 0;
  position: relative;
}
.trip-card-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.cover-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent, #10ac84);
  color: #fff;
  font-size: 1.4rem;
  font-weight: 700;
  letter-spacing: 0.5px;
}
.trip-badge {
  position: absolute;
  bottom: 4px;
  right: 4px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  font-size: 0.6rem;
  padding: 1px 6px;
  border-radius: 8px;
}

.trip-card-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 4px;
  min-width: 0;
}
.trip-card-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.trip-card-meta {
  display: flex;
  gap: 8px;
  font-size: 0.7rem;
  color: var(--text-muted);
}
.trip-card-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
  flex-wrap: wrap;
}
.card-ai-btn {
  font-size: 0.72rem;
  color: #fff;
  padding: 4px 10px;
  border-radius: 8px;
  background: var(--accent-2, #0a84d0);
  box-shadow: 0 2px 6px rgba(10, 132, 208, 0.2);
  font-weight: 500;
}
.card-ai-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 3px 10px rgba(10, 132, 208, 0.3);
}
.card-view-btn {
  font-size: 0.72rem;
  color: var(--accent);
  padding: 4px 10px;
  border-radius: 8px;
  background: rgba(99, 102, 241, 0.1);
  font-weight: 500;
}
.card-view-btn:hover {
  background: rgba(99, 102, 241, 0.15);
}
.card-delete-btn {
  font-size: 0.72rem;
  color: var(--text-muted);
  padding: 4px 8px;
  border-radius: 8px;
  margin-left: 4px;
  transition: all 0.15s ease;
}
.card-delete-btn:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

.manual-badge {
  position: absolute;
  top: 4px;
  left: 4px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6rem;
}

.empty-state, .loading-state {
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

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 10px;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.loading-text {
  font-size: 0.8rem;
  color: var(--text-secondary);
}

/* 详情弹窗 */
.trip-detail-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
  z-index: 999;
  display: flex;
  align-items: center;
  justify-content: center;
}
.trip-detail-modal {
  width: 90%;
  max-width: 600px;
  max-height: 85vh;
  background: var(--bg-panel-solid);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.detail-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px 6px;
}
.detail-actions {
  display: flex;
  gap: 8px;
  padding: 0 16px 10px;
  flex-wrap: wrap;
  border-bottom: 1px solid var(--border);
}
.detail-back {
  font-size: 1.2rem;
  color: var(--text-secondary);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
}
.detail-back:hover {
  background: var(--bg-hover);
}
.detail-title {
  flex: 1;
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-primary);
}
.detail-journal-btn {
  font-size: 0.75rem;
  color: #fff;
  padding: 5px 12px;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent-2), #7c3aed);
}
.detail-meta {
  padding: 8px 16px;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.journal-editor {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}
.journal-textarea {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px;
  font-family: inherit;
  font-size: 0.85rem;
  resize: vertical;
  outline: none;
  transition: border-color 0.2s;
}
.journal-textarea:focus {
  border-color: var(--accent);
}
.journal-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}
.char-count {
  font-size: 0.7rem;
  color: var(--text-muted);
}
.toolbar-btns {
  display: flex;
  gap: 8px;
}
.save-btn {
  font-size: 0.75rem;
  color: #fff;
  padding: 4px 12px;
  border-radius: 8px;
  background: var(--accent);
}
.polish-btn {
  font-size: 0.72rem;
  color: #fff;
  padding: 4px 10px;
  border-radius: 8px;
  background: var(--accent-2, #0a84d0);
  box-shadow: 0 2px 6px rgba(10, 132, 208, 0.2);
  font-weight: 500;
}
.polish-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 3px 10px rgba(10, 132, 208, 0.3);
}
.polish-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.polish-progress {
  margin-top: 8px;
  padding: 8px 12px;
  background: rgba(192, 133, 82, 0.08);
  border-radius: 8px;
}
.ai-status-small {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.75rem;
  color: var(--accent-2);
}
.ai-spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid var(--accent-2-glow);
  border-top-color: var(--accent-2);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.journal-display {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  max-height: 200px;
  overflow-y: auto;
}
.journal-text {
  font-size: 0.85rem;
  line-height: 1.7;
  color: var(--text-secondary);
}
.journal-photo-placeholder {
  display: block;
  text-align: center;
  padding: 20px;
  color: var(--text-muted);
  font-size: 0.8rem;
}
.journal-photo {
  margin: 16px auto;
  max-width: 100%;
  text-align: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  overflow: hidden;
  transition: transform 0.2s ease;
}
.journal-photo:hover {
  transform: scale(1.02);
}
.journal-photo img {
  max-width: 100%;
  max-height: 320px;
  object-fit: contain;
  border-radius: var(--radius-sm);
  display: block;
  margin: 0 auto;
}
.journal-photo-caption {
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-top: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
}
.journal-photo-num {
  color: var(--text-muted);
}
.journal-locate-btn {
  color: var(--accent);
  cursor: pointer;
  padding: 2px 8px;
  border-radius: 10px;
  background: rgba(14, 165, 233, 0.08);
  transition: background 0.15s ease;
}
.journal-locate-btn:hover {
  background: rgba(14, 165, 233, 0.18);
}
.journal-photo.flash {
  animation: journal-flash 1.8s ease;
}
@keyframes journal-flash {
  0%, 100% { box-shadow: none; }
  20% { box-shadow: 0 0 0 3px var(--accent), 0 0 16px var(--accent-glow); border-radius: var(--radius-sm); }
}
.journal-empty {
  padding: 20px 16px;
  text-align: center;
  font-size: 0.8rem;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
}
.journal-source {
  display: inline-block;
  margin-left: 6px;
  font-size: 0.65rem;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(99, 102, 241, 0.1);
  color: var(--accent-2);
}
.journal-source.polished {
  background: rgba(192, 133, 82, 0.1);
  color: var(--accent-2);
}

/* AI 生成中 */
.ai-generating {
  padding: 16px;
  border-bottom: 1px solid var(--border);
  max-height: 300px;
  overflow-y: auto;
}
.ai-status {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  font-size: 0.8rem;
  color: var(--accent-2);
}
.ai-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent-2);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
.ai-stream-text {
  font-size: 0.85rem;
  line-height: 1.8;
  color: var(--text-secondary);
  white-space: pre-wrap;
}
.cursor-blink {
  color: var(--accent-2);
  animation: blink 1s step-end infinite;
}
@keyframes blink {
  50% { opacity: 0; }
}
.ai-btn {
  background: #0a84d0 !important;
}
.ai-btn:hover {
  box-shadow: 0 2px 12px rgba(10, 132, 208, 0.3);
}

.detail-photos {
  flex: 1;
  overflow-y: auto;
  padding: 8px 16px;
}

.detail-photo-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
}
.detail-photo-item:hover {
  background: var(--bg-hover);
  border-color: var(--border-glow);
}
.detail-photo-item.flash {
  animation: photo-flash 1.8s ease;
  border-color: var(--accent);
}
@keyframes photo-flash {
  0%, 100% { background: transparent; }
  20% { background: rgba(14, 165, 233, 0.12); }
}
.journal-link-btn {
  flex-shrink: 0;
  font-size: 0.68rem;
  color: var(--accent);
  background: rgba(14, 165, 233, 0.08);
  padding: 3px 8px;
  border-radius: 10px;
  white-space: nowrap;
  transition: background 0.15s ease;
}
.journal-link-btn:hover {
  background: rgba(14, 165, 233, 0.18);
}
.photo-item-thumb {
  width: 48px;
  height: 48px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-card);
}
.photo-item-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.photo-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  color: var(--text-muted);
  background: var(--bg-card);
}
.photo-item-info {
  flex: 1;
  min-width: 0;
}
.photo-item-info .item-name {
  font-size: 0.78rem;
  color: var(--text-primary);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.photo-item-info .item-meta {
  margin-top: 2px;
  display: flex;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--text-muted);
}
.photo-item-info .item-date,
.photo-item-info .item-city {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.photo-item-arrow {
  color: var(--text-muted);
  font-size: 1rem;
  flex-shrink: 0;
}

.photo-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}
.photo-pagination .page-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.72rem;
  color: var(--text-secondary);
  background: var(--bg-hover);
  transition: all 0.15s ease;
}
.photo-pagination .page-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.photo-pagination .page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.photo-pagination .page-info {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.detail-map-btn {
  margin: 0 16px 14px;
  padding: 10px;
  border-radius: var(--radius-sm);
  background: var(--accent, #10ac84);
  color: #fff;
  font-size: 0.8rem;
  font-weight: 500;
  text-align: center;
}
.detail-map-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(16, 172, 132, 0.25);
}

/* 照片列表头部 */
.photos-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 6px;
  padding: 10px 16px 6px;
  border-top: 1px solid var(--border);
}
.photos-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
}
.photos-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.add-photo-btn {
  font-size: 0.72rem;
  color: var(--accent);
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(16, 172, 132, 0.1);
  font-weight: 500;
  transition: all 0.15s ease;
}
.add-photo-btn:hover {
  background: rgba(99, 102, 241, 0.2);
}

/* 移除照片按钮 */
.remove-photo-btn {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--bg-hover);
  color: var(--text-muted);
  font-size: 0.7rem;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  opacity: 0;
  transition: all 0.15s ease;
}
.detail-photo-item:hover .remove-photo-btn {
  opacity: 1;
}
.remove-photo-btn:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

/* 照片选择器弹窗 */
.photo-selector-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}
.photo-selector-modal {
  width: 90%;
  max-width: 500px;
  max-height: 80vh;
  background: var(--bg-panel-solid);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.selector-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}
.selector-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}
.selector-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-secondary);
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.selector-close:hover {
  background: var(--bg-hover);
}

.selector-stats {
  padding: 8px 16px;
  font-size: 0.75rem;
  color: var(--text-muted);
  background: var(--bg-card);
  border-bottom: 1px solid var(--border);
}

.selector-photos {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}
.selector-photo-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 2px solid transparent;
  margin-bottom: 4px;
}
.selector-photo-item:hover {
  background: var(--bg-hover);
}
.selector-photo-item.selected {
  background: rgba(99, 102, 241, 0.1);
  border-color: var(--accent);
}
.selector-photo-thumb {
  width: 44px;
  height: 44px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--bg-card);
}
.selector-photo-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.selector-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  color: var(--text-muted);
  background: var(--bg-card);
}
.selector-photo-info {
  flex: 1;
  min-width: 0;
}
.selector-photo-name {
  font-size: 0.78rem;
  color: var(--text-primary);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.selector-photo-meta {
  margin-top: 2px;
  display: flex;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--text-muted);
}
.selector-photo-meta .selector-date,
.selector-photo-meta .selector-city {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.selector-check {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  border: 2px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 0.7rem;
  color: transparent;
  transition: all 0.15s ease;
}
.selector-photo-item.selected .selector-check {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.selector-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 30px;
  gap: 10px;
  color: var(--text-muted);
  font-size: 0.8rem;
}

.selector-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}
.selector-pagination .page-btn {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 0.72rem;
  color: var(--text-secondary);
  background: var(--bg-hover);
  transition: all 0.15s ease;
}
.selector-pagination .page-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}
.selector-pagination .page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.selector-pagination .page-info {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.selector-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}
.select-all-btn {
  font-size: 0.72rem;
  color: var(--text-secondary);
  padding: 5px 10px;
  border-radius: 6px;
  background: var(--bg-hover);
}
.select-all-btn:hover {
  color: var(--accent);
}
.footer-actions {
  display: flex;
  gap: 8px;
}
.cancel-btn {
  font-size: 0.75rem;
  color: var(--text-secondary);
  padding: 6px 14px;
  border-radius: 6px;
  background: var(--bg-hover);
}
.cancel-btn:hover {
  background: var(--border);
}
.confirm-btn {
  font-size: 0.75rem;
  color: #fff;
  padding: 6px 14px;
  border-radius: 6px;
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  font-weight: 500;
}
.confirm-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px var(--accent-glow);
}
.confirm-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 新建游记弹窗 */
.create-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}
.create-modal {
  width: 90%;
  max-width: 420px;
  background: var(--bg-panel-solid);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}
.create-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}
.create-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}
.create-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-secondary);
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.create-close:hover {
  background: var(--bg-hover);
}

.create-body {
  padding: 16px;
}
.form-item {
  margin-bottom: 14px;
}
.form-item:last-child {
  margin-bottom: 0;
}
.form-label {
  display: block;
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 6px;
  font-weight: 500;
}
.form-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 0.8rem;
  color: var(--text-primary);
  background: var(--bg-card);
  outline: none;
  transition: border-color 0.2s;
  box-sizing: border-box;
}
.form-input:focus {
  border-color: var(--accent);
}
.form-row {
  display: flex;
  gap: 12px;
}
.form-row .form-item {
  flex: 1;
}
.form-hint {
  margin-top: 12px;
  font-size: 0.7rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.create-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
}

/* 生成影集按钮 */
.slideshow-btn {
  font-size: 0.72rem;
  color: #fff;
  padding: 4px 10px;
  border-radius: 6px;
  background: var(--accent-2, #0a84d0);
  font-weight: 500;
  transition: all 0.15s ease;
  margin-left: 8px;
}
.slideshow-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(10, 132, 208, 0.3);
}
.slideshow-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 影集生成弹窗 */
.slideshow-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}
.slideshow-modal {
  width: 90%;
  max-width: 520px;
  max-height: 85vh;
  background: var(--bg-panel-solid);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.slideshow-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
}
.slideshow-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}
.slideshow-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-secondary);
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.slideshow-close:hover {
  background: var(--bg-hover);
}

.slideshow-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.form-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.form-label-row .form-label {
  margin-bottom: 0;
}

.ai-gen-narration-btn {
  font-size: 0.7rem;
  color: #fff;
  padding: 3px 10px;
  border-radius: 6px;
  background: var(--accent-2, #0a84d0);
  font-weight: 500;
}
.ai-gen-narration-btn:hover:not(:disabled) {
  box-shadow: 0 2px 6px rgba(10, 132, 208, 0.3);
}
.ai-gen-narration-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.narration-textarea {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 10px;
  font-family: inherit;
  font-size: 0.82rem;
  resize: vertical;
  outline: none;
  transition: border-color 0.2s;
  box-sizing: border-box;
  line-height: 1.6;
}
.narration-textarea:focus {
  border-color: var(--accent);
}

.narration-hint {
  margin-top: 4px;
  font-size: 0.7rem;
  color: var(--text-muted);
  text-align: right;
}

.params-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.param-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.param-label {
  font-size: 0.72rem;
  color: var(--text-muted);
}
.param-select {
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 0.78rem;
  color: var(--text-primary);
  background: var(--bg-card);
  outline: none;
  cursor: pointer;
}
.param-select:focus {
  border-color: var(--accent);
}

.music-selector {
  display: flex;
  gap: 8px;
}
.music-input {
  flex: 1;
}
.select-music-btn {
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--accent);
  background: rgba(99, 102, 241, 0.1);
  font-weight: 500;
  white-space: nowrap;
  flex-shrink: 0;
}
.select-music-btn:hover {
  background: rgba(99, 102, 241, 0.2);
}
.music-clear {
  margin-top: 4px;
  text-align: right;
}
.music-clear button {
  font-size: 0.7rem;
  color: var(--text-muted);
}
.music-clear button:hover {
  color: #ef4444;
}

.slideshow-info {
  display: flex;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--bg-card);
  border-radius: 8px;
  margin-top: 12px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

/* 生成中状态 */
.slideshow-generating {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}
.generating-icon {
  font-size: 3rem;
  margin-bottom: 16px;
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.1); opacity: 0.8; }
}
.generating-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 16px;
}
.generating-progress-bar {
  width: 80%;
  height: 8px;
  background: var(--bg-hover);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 8px;
}
.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-2));
  border-radius: 4px;
  transition: width 0.3s ease;
}
.generating-progress-text {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--accent);
  margin-bottom: 8px;
}
.generating-hint {
  font-size: 0.75rem;
  color: var(--text-muted);
}

/* 生成结果 */
.slideshow-result {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}
.result-icon {
  font-size: 3rem;
  margin-bottom: 16px;
}
.result-title {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}
.result-path {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-bottom: 20px;
  text-align: center;
  word-break: break-all;
  padding: 0 20px;
}
.result-actions {
  display: flex;
  gap: 12px;
}
.result-btn {
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 0.8rem;
  font-weight: 500;
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.result-btn:hover {
  background: var(--border);
}
.result-btn.primary {
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  color: #fff;
}
.result-btn.primary:hover {
  box-shadow: 0 2px 8px var(--accent-glow);
}

.slideshow-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
  flex-shrink: 0;
}

.photos-actions {
  display: flex;
  align-items: center;
}
</style>
