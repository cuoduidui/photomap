<template>
  <div class="dialog-overlay" @click="emit('close')">
    <div class="dialog-card" @click.stop>
      <div class="dialog-header">
        <div class="header-icon">📍</div>
        <h3>{{ $t("locationDialog.title") }}</h3>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="dialog-body">
        <p class="dialog-info" v-html="$t('locationDialog.selectedCountHtml', { n: store.selectedPhotoIds.length })"></p>

        <!-- 搜索 -->
        <div class="section">
          <div class="section-label">
            <span class="label-dot"></span>
            {{ $t("locationDialog.addressSearch") }}
          </div>
          <div class="search-section">
            <input type="text" v-model="searchKeyword" :placeholder="$t('locationDialog.searchPlaceholder')"
              @keyup.enter="doSearch" class="search-input" />
            <button class="btn btn-primary" @click="doSearch" :disabled="!searchKeyword.trim()">
              {{ $t("locationDialog.search") }}
            </button>
          </div>
        </div>

        <!-- 手动坐标 -->
        <div class="section">
          <div class="section-label">
            <span class="label-dot"></span>
            {{ $t("locationDialog.manualCoords") }}
          </div>
          <div class="coords-inputs">
            <input type="number" step="0.000001" v-model.number="manualLat" :placeholder="$t('locationDialog.latPlaceholder')" class="coord-input" />
            <input type="number" step="0.000001" v-model.number="manualLng" :placeholder="$t('locationDialog.lngPlaceholder')" class="coord-input" />
            <button class="btn btn-ghost" @click="useManualCoords" :disabled="!canUseManual">
              {{ $t("locationDialog.use") }}
            </button>
          </div>
        </div>

        <!-- 搜索结果 -->
        <div v-if="searchResults.length" class="search-results">
          <div v-for="(r, i) in searchResults" :key="i" class="result-item"
            :class="{ selected: selectedResult === r }"
            @click="selectResult(r)">
            <div class="result-main">
              <div class="result-name">{{ r.name }}</div>
              <div class="result-addr">{{ r.address || $t('locationDialog.noAddress') }}</div>
            </div>
            <span v-if="selectedResult === r" class="result-check">✓</span>
          </div>
        </div>

        <div v-if="searchError" class="error-text">{{ searchError }}</div>

        <!-- 快捷地点 -->
        <div class="section">
          <div class="section-label">
            <span class="label-dot"></span>
            {{ $t("locationDialog.quickLocations") }}
          </div>
          <div v-if="store.customLocations.length" class="quick-locations">
            <button v-for="loc in store.customLocations" :key="loc.id"
              class="quick-btn"
              @click="useCustomLocation(loc)">
              {{ loc.name }}
            </button>
          </div>
          <p v-else class="empty-text">{{ $t("locationDialog.noQuickLocations") }}</p>
        </div>

        <!-- 确认区域 -->
        <div v-if="selectedResult" class="confirm-section">
          <div class="confirm-info">
            <div class="confirm-name">{{ selectedResult.name }}</div>
            <div class="confirm-coords">
              <span class="coord-pair">
                <span class="coord-label">{{ $t("locationDialog.latitude") }}</span>
                <span class="coord-val">{{ selectedResult.latitude?.toFixed(6) }}</span>
              </span>
              <span class="coord-pair">
                <span class="coord-label">{{ $t("locationDialog.longitude") }}</span>
                <span class="coord-val">{{ selectedResult.longitude?.toFixed(6) }}</span>
              </span>
            </div>
          </div>
          <button class="btn btn-accent confirm-btn" @click="confirmLocation">
            {{ $t("locationDialog.confirm") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { searchAddress } from "../utils/tauri";

const emit = defineEmits(["close", "done"]);
const store = usePhotoStore();
const { t } = useI18n();

const searchKeyword = ref("");
const searchResults = ref([]);
const selectedResult = ref(null);
const searchError = ref("");
const manualLat = ref(null);
const manualLng = ref(null);

const canUseManual = computed(() => {
  return manualLat.value != null && manualLng.value != null &&
    !isNaN(manualLat.value) && !isNaN(manualLng.value) &&
    manualLat.value >= -90 && manualLat.value <= 90 &&
    manualLng.value >= -180 && manualLng.value <= 180;
});

function useManualCoords() {
  if (!canUseManual.value) return;
  selectedResult.value = {
    name: t("locationDialog.manualCoordsName"),
    address: `(${manualLat.value.toFixed(6)}, ${manualLng.value.toFixed(6)})`,
    latitude: manualLat.value,
    longitude: manualLng.value,
  };
}

async function doSearch() {
  if (!searchKeyword.value.trim()) return;
  searchError.value = "";
  searchResults.value = [];
  selectedResult.value = null;

  try {
    const results = await searchAddress(searchKeyword.value.trim());
    if (results.length === 0) {
      searchError.value = t("locationDialog.notFound");
    } else {
      searchResults.value = results;
    }
  } catch (e) {
    searchError.value = t("locationDialog.searchFailed", { error: e });
  }
}

function selectResult(result) {
  selectedResult.value = result;
}

function useCustomLocation(loc) {
  selectedResult.value = {
    name: loc.name,
    address: loc.address,
    latitude: loc.latitude,
    longitude: loc.longitude,
  };
}

async function confirmLocation() {
  if (!selectedResult.value || !store.selectedPhotoIds.length) return;

  try {
    await store.updateLocation(
      store.selectedPhotoIds,
      selectedResult.value.latitude,
      selectedResult.value.longitude,
      selectedResult.value.address
    );
    emit("done");
  } catch (e) {
    searchError.value = t("locationDialog.markFailed", { error: e });
  }
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  animation: fadeIn 0.2s ease;
}
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.dialog-card {
  background: var(--bg-panel-solid);
  border: 1px solid var(--border-glow);
  border-radius: 12px;
  width: 480px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px var(--accent-glow);
  animation: slideUp 0.3s ease;
}
@keyframes slideUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  position: relative;
}
.header-icon { font-size: 1.2rem; }
.dialog-header h3 {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
  flex: 1;
}
.close-btn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 0.8rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.close-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.dialog-body {
  padding: 16px 20px 20px;
}

.dialog-info {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-bottom: 16px;
}
.dialog-info strong {
  color: var(--accent);
  font-size: 1rem;
}

.section {
  margin-bottom: 16px;
}
.section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 8px;
}
.label-dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 6px var(--accent-glow);
}

.search-section {
  display: flex;
  gap: 8px;
}

.search-input,
.coord-input {
  flex: 1;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 0.85rem;
  transition: all 0.2s ease;
}
.search-input:focus,
.coord-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1);
}
.search-input::placeholder,
.coord-input::placeholder {
  color: var(--text-muted);
}

.coords-inputs {
  display: flex;
  gap: 8px;
}
.coord-input {
  font-size: 0.82rem;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 14px;
  border-radius: var(--radius-sm);
  font-size: 0.8rem;
  font-weight: 500;
  transition: all 0.2s ease;
}
.btn-primary {
  background: linear-gradient(135deg, var(--accent), #0284c7);
  color: #fff;
  box-shadow: 0 2px 8px var(--accent-glow);
}
.btn-primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px var(--accent-glow);
}
.btn-ghost {
  background: var(--bg-card);
  color: var(--text-secondary);
  border: 1px solid var(--border);
}
.btn-ghost:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--border-glow);
}
.btn-accent {
  background: linear-gradient(135deg, var(--accent-2), #7c3aed);
  color: #fff;
  box-shadow: 0 2px 8px var(--accent-2-glow);
}
.btn-accent:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 14px var(--accent-2-glow);
}
.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.search-results {
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  margin-bottom: 16px;
  background: var(--bg-card);
}
.result-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  transition: background 0.15s ease;
}
.result-item:last-child { border-bottom: none; }
.result-item:hover {
  background: var(--bg-hover);
}
.result-item.selected {
  background: rgba(14, 165, 233, 0.1);
}
.result-name {
  font-size: 0.82rem;
  color: var(--text-primary);
  margin-bottom: 2px;
}
.result-addr {
  font-size: 0.72rem;
  color: var(--text-muted);
}
.result-check {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  font-size: 0.7rem;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 8px var(--accent-glow);
}

.error-text {
  color: #f87171;
  font-size: 0.8rem;
  padding: 8px 12px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: var(--radius-sm);
  margin-bottom: 12px;
}

.quick-locations {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.quick-btn {
  padding: 5px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  color: var(--text-secondary);
  font-size: 0.75rem;
  transition: all 0.15s ease;
}
.quick-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(14, 165, 233, 0.08);
}

.empty-text {
  font-size: 0.78rem;
  color: var(--text-muted);
  padding: 8px 0;
}

.confirm-section {
  margin-top: 16px;
  padding: 14px;
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.1), rgba(14, 165, 233, 0.1));
  border: 1px solid var(--border-glow);
  border-radius: var(--radius);
}
.confirm-info {
  margin-bottom: 12px;
}
.confirm-name {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}
.confirm-coords {
  display: flex;
  gap: 16px;
}
.coord-pair {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.coord-label {
  font-size: 0.65rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.coord-val {
  font-size: 0.8rem;
  color: var(--accent);
  font-family: monospace;
}
.confirm-btn {
  width: 100%;
  padding: 10px;
}
</style>
