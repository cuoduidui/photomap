<template>
  <div class="dialog-overlay" @click="emit('close')">
    <div class="dialog-card" @click.stop>
      <div class="dialog-header">
        <h3>{{ $t("settings.title") }}</h3>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="dialog-body">
        <!-- 高德地图 API -->
        <div class="setting-section">
          <h4>{{ $t("settings.amapTitle") }}</h4>
          <p class="setting-desc">
            {{ $t("settings.amapDescBefore") }}
            <a href="https://lbs.amap.com/" target="_blank" rel="noopener">{{ $t("settings.amapLink") }}</a>
            {{ $t("settings.amapDescAfter") }}
          </p>
          <div class="input-row">
            <input type="text" v-model="apiKey" :placeholder="$t('settings.apiKeyPlaceholder')" class="api-input" />
            <button class="save-btn" @click="saveApiKey">{{ $t("common.save") }}</button>
          </div>
          <div v-if="saveMsg" class="save-msg" :class="{ error: saveError }">{{ saveMsg }}</div>

          <!-- 批量逆地理编码 -->
          <div class="geocode-section">
            <button class="geocode-btn" @click="doBatchGeocode" 
              :disabled="geocoding || !apiKey.trim()">
              <span v-if="geocoding">
                <span class="btn-spinner"></span>
                {{ $t("settings.geocodingProgress", { done: geocodeDone, total: geocodeTotal }) }}
              </span>
              <span v-else>📍 {{ $t("settings.batchGeocode") }}</span>
            </button>
            <button v-if="geocoding" class="geocode-cancel-btn" @click="cancelGeocode">{{ $t("common.cancel") }}</button>
            <p class="geocode-desc">
              {{ $t("settings.geocodeDesc") }}
            </p>
            <label class="config-row">
              <input type="checkbox" v-model="geocodeForce" />
              <span>{{ $t("settings.refreshAll") }}</span>
            </label>
            <div v-if="geocodeResult" class="geocode-result">
              ✅ {{ $t("settings.geocodeResult", { n: geocodeResult }) }}
            </div>
          </div>

          <!-- 重新生成缩略图 -->
          <div class="geocode-section">
            <button class="geocode-btn" @click="doRegenerateThumbs"
              :disabled="regenerating || store.photos.length === 0">
              <span v-if="regenerating">{{ $t("settings.regeneratingProgress", { done: thumbDone, total: thumbTotal }) }}</span>
              <span v-else>🖼️ {{ $t("settings.regenerateThumbs") }}</span>
            </button>
            <button v-if="regenerating" class="geocode-cancel-btn" @click="cancelGeocode">{{ $t("common.cancel") }}</button>
            <p class="geocode-desc">
              {{ $t("settings.regenerateDesc") }}
            </p>
            <div v-if="thumbResult != null" class="geocode-result">
              ✅ {{ $t("settings.thumbResult", { n: thumbResult }) }}
            </div>
          </div>
        </div>

        <!-- AI 游记配置 -->
        <div class="setting-section">
          <h4>{{ $t("settings.aiTitle") }}</h4>
          <p class="setting-desc">{{ $t("settings.aiDesc") }}</p>

          <div class="config-group">
            <label class="config-label">{{ $t("settings.aiProvider") }}</label>
            <select v-model="aiProvider" class="config-select">
              <option value="openai">{{ $t("settings.aiProviderOpenAI") }}</option>
              <option value="deepseek">DeepSeek</option>
              <option value="qwen">{{ $t("settings.aiProviderQwen") }}</option>
              <option value="ollama">{{ $t("settings.aiProviderOllama") }}</option>
            </select>
          </div>

          <div class="config-group">
            <label class="config-label">API Key</label>
            <input type="password" v-model="aiApiKey" :placeholder="$t('settings.aiApiKeyPlaceholder')"
              class="api-input" />
            <p v-if="aiProvider === 'ollama'" class="config-hint">
              {{ $t("settings.ollamaHint") }}
            </p>
          </div>

          <div class="config-group">
            <label class="config-label">{{ $t("settings.modelName") }}</label>
            <input type="text" v-model="aiModel" :placeholder="modelPlaceholder"
              class="api-input" />
          </div>

          <div class="config-group">
            <label class="config-label">{{ $t("settings.apiUrlOptional") }}</label>
            <input type="text" v-model="aiBaseUrl" :placeholder="$t('settings.apiUrlPlaceholder')"
              class="api-input" />
          </div>

          <button class="save-btn" @click="saveAiConfig">{{ $t("settings.saveAiConfig") }}</button>
          <div v-if="aiSaveMsg" class="save-msg" :class="{ error: aiSaveError }">{{ aiSaveMsg }}</div>
        </div>

        <!-- 外观 / 皮肤 -->
        <div class="setting-section">
          <h4>{{ $t("settings.appearanceTitle") }}</h4>
          <p class="setting-desc">{{ $t("settings.appearanceDesc") }}</p>
          <ThemeSwitcher :current="currentTheme" @select="onSelectTheme" />
        </div>

        <!-- 快捷地点 -->
        <div class="setting-section">
          <h4>{{ $t("settings.quickLocationsTitle") }}</h4>
          <div class="add-location">
            <input type="text" v-model="newLocName" :placeholder="$t('settings.locationNamePlaceholder')" />
            <input type="number" v-model="newLocLat" :placeholder="$t('settings.latitudePlaceholder')" step="0.0001" />
            <input type="number" v-model="newLocLng" :placeholder="$t('settings.longitudePlaceholder')" step="0.0001" />
            <button class="save-btn" @click="addLocation">{{ $t("settings.add") }}</button>
          </div>
          <div v-if="store.customLocations.length" class="loc-list">
            <div v-for="loc in store.customLocations" :key="loc.id" class="loc-item">
              <span>{{ loc.name }} ({{ loc.latitude.toFixed(4) }}, {{ loc.longitude.toFixed(4) }})</span>
              <button class="del-btn" @click="store.removeCustomLocation(loc.id)">{{ $t("common.delete") }}</button>
            </div>
          </div>
          <p v-else class="empty-text">{{ $t("settings.noQuickLocations") }}</p>
        </div>

        <!-- 语言 -->
        <div class="setting-section">
          <h4>{{ $t("settings.languageTitle") }}</h4>
          <p class="setting-desc">{{ $t("settings.languageDesc") }}</p>
          <select v-model="currentLocale" class="config-select" @change="onLocaleChange">
            <option value="zh-CN">中文</option>
            <option value="en">English</option>
            <option value="ja">日本語</option>
            <option value="fr">Français</option>
            <option value="ko">한국어</option>
            <option value="de">Deutsch</option>
            <option value="ru">Русский</option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { getConfig, setConfig, onGeocodeProgress } from "../utils/tauri";
import ThemeSwitcher from "./ThemeSwitcher.vue";
import { loadTheme, setTheme, THEMES } from "../utils/theme";
import { setLocale, getLocale } from "../i18n";

const emit = defineEmits(["close"]);
const store = usePhotoStore();
const { t } = useI18n();
const currentLocale = ref(getLocale());

const apiKey = ref("");
const saveMsg = ref("");
const saveError = ref(false);
const newLocName = ref("");
const newLocLat = ref(null);
const newLocLng = ref(null);

const aiProvider = ref("openai");
const aiApiKey = ref("");
const aiModel = ref("");
const aiBaseUrl = ref("");
const aiSaveMsg = ref("");
const aiSaveError = ref(false);

const currentTheme = ref("fresh");

// 批量逆地理编码
const geocoding = ref(false);
const geocodeDone = ref(0);
const geocodeTotal = ref(0);
const geocodeResult = ref(null);
const geocodeForce = ref(false);
const regenerating = ref(false);
const thumbDone = ref(0);
const thumbTotal = ref(0);
const thumbResult = ref(null);
let unlistenGeocode = null;

async function doBatchGeocode() {
  if (geocoding.value || !apiKey.value.trim()) return;
  geocoding.value = true;
  geocodeDone.value = 0;
  geocodeTotal.value = 0;
  geocodeResult.value = null;
  
  try {
    const updated = await store.runBatchGeocode(geocodeForce.value);
    geocodeResult.value = updated;
  } catch (e) {
    console.error("逆地理编码失败:", e);
  } finally {
    geocoding.value = false;
  }
}

async function cancelGeocode() {
  try {
    const { cancelLongTask } = await import("../utils/tauri");
    await cancelLongTask();
  } catch (e) {
    console.warn("取消失败:", e);
  }
}

async function doRegenerateThumbs() {
  if (regenerating.value) return;
  regenerating.value = true;
  thumbDone.value = 0;
  thumbTotal.value = store.photos.length;
  thumbResult.value = null;
  let unlistenThumb = null;
  try {
    const { onImportProgress, regenerateThumbnails, clearImageCache } = await import("../utils/tauri");
    unlistenThumb = await onImportProgress((done, total) => {
      if (done < 0) return;
      thumbDone.value = done;
      thumbTotal.value = total;
    });
    const count = await regenerateThumbnails();
    thumbResult.value = count;
    clearImageCache();
    await store.loadPhotos();
    store.importProgress = null;
  } catch (e) {
    console.error("重新生成缩略图失败:", e);
  } finally {
    if (unlistenThumb) unlistenThumb();
    regenerating.value = false;
  }
}

const modelPlaceholder = computed(() => {
  switch (aiProvider.value) {
    case "openai": return "gpt-4o-mini";
    case "deepseek": return "deepseek-chat";
    case "qwen": return "qwen-plus";
    case "ollama": return "qwen2.5:7b";
    default: return t("settings.modelNameDefault");
  }
});

function onLocaleChange() {
  setLocale(currentLocale.value);
}

onMounted(async () => {
  const stored = await getConfig("amap_api_key");
  apiKey.value = stored || "";

  aiProvider.value = (await getConfig("ai_provider")) || "openai";
  aiApiKey.value = (await getConfig("ai_api_key")) || "";
  aiModel.value = (await getConfig("ai_model")) || "";
  aiBaseUrl.value = (await getConfig("ai_base_url")) || "";

  currentTheme.value = await loadTheme();

  // 监听逆地理编码进度
  unlistenGeocode = await onGeocodeProgress((data) => {
    if (data && data.length >= 2) {
      geocodeDone.value = data[0];
      geocodeTotal.value = data[1];
    }
  });
});

onUnmounted(() => {
  if (unlistenGeocode) unlistenGeocode();
});

async function saveApiKey() {
  saveMsg.value = "";
  saveError.value = false;
  try {
    await store.setApiKey(apiKey.value.trim());
    saveMsg.value = t("settings.apiKeySaved");
  } catch (e) {
    saveMsg.value = t("settings.saveFailed", { error: e });
    saveError.value = true;
  }
}

async function saveAiConfig() {
  aiSaveMsg.value = "";
  aiSaveError.value = false;
  try {
    await setConfig("ai_provider", aiProvider.value);
    await setConfig("ai_api_key", aiApiKey.value);
    await setConfig("ai_model", aiModel.value);
    await setConfig("ai_base_url", aiBaseUrl.value);
    aiSaveMsg.value = t("settings.aiConfigSaved");
  } catch (e) {
    aiSaveMsg.value = t("settings.saveFailed", { error: e });
    aiSaveError.value = true;
  }
}

async function onSelectTheme(id) {
  currentTheme.value = await setTheme(id);
}

async function addLocation() {
  if (!newLocName.value.trim() || !newLocLat.value || !newLocLng.value) return;
  try {
    await store.addCustomLocation(
      newLocName.value.trim(),
      parseFloat(newLocLat.value),
      parseFloat(newLocLng.value),
      null
    );
    newLocName.value = "";
    newLocLat.value = null;
    newLocLng.value = null;
  } catch (e) {
    saveMsg.value = t("settings.addFailed", { error: e });
    saveError.value = true;
  }
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
  width: 520px;
  max-height: 600px;
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
.dialog-header h3 { font-size: 1.05rem; color: var(--ink); }
.close-btn {
  width: 28px; height: 28px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: var(--text-muted);
}
.close-btn:hover { background: var(--bg-hover); }

.dialog-body { padding: 1rem 1.5rem; }

.setting-section { margin-bottom: 1.5rem; }
.setting-section h4 {
  font-size: 0.95rem;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.setting-desc {
  font-size: 0.8rem;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}
.setting-desc a { color: var(--accent); }

.input-row {
  display: flex;
  gap: 0.5rem;
}
.api-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--rule);
  border-radius: 8px;
  font-size: 0.85rem;
  outline: none;
}
.api-input:focus { border-color: var(--accent); }

.config-group {
  margin-bottom: 0.75rem;
}
.config-label {
  display: block;
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-bottom: 4px;
}
.config-select {
  width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--rule);
  border-radius: 8px;
  font-size: 0.85rem;
  outline: none;
  background: var(--bg2);
}
.config-hint {
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-top: 3px;
}

.save-btn {
  padding: 5px 14px;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent), var(--accent2));
  color: var(--bg2);
  font-size: 0.8rem;
  font-weight: 500;
}
.save-btn:hover { transform: translateY(-1px); }

.del-btn {
  font-size: 0.75rem;
  color: var(--danger);
  padding: 2px 8px;
}

.save-msg {
  margin-top: 0.4rem;
  font-size: 0.8rem;
  color: var(--success);
}
.save-msg.error { color: var(--danger); }

.geocode-section {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--rule);
}
.geocode-btn {
  width: 100%;
  padding: 10px;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent2), var(--accent));
  color: var(--bg2);
  font-size: 0.82rem;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.2s ease;
}
.geocode-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px var(--accent2-soft);
}
.geocode-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.geocode-cancel-btn {
  margin-top: 8px;
  width: 100%;
  padding: 8px;
  border-radius: 8px;
  font-size: 0.78rem;
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}
.geocode-cancel-btn:hover {
  background: rgba(248, 113, 113, 0.2);
}
.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: var(--bg2);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.geocode-desc {
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-top: 6px;
  line-height: 1.4;
}
.geocode-result {
  margin-top: 8px;
  padding: 6px 10px;
  background: var(--success-soft);
  border: 1px solid var(--success-soft);
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--success);
  text-align: center;
}

.add-location {
  display: flex;
  gap: 0.4rem;
  flex-wrap: wrap;
}
.add-location input {
  flex: 1; min-width: 100px;
  padding: 6px 10px;
  border: 1px solid var(--rule);
  border-radius: 8px;
  font-size: 0.85rem;
  outline: none;
}

.loc-list { margin-top: 0.5rem; }
.loc-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0;
  border-bottom: 1px solid var(--rule);
  font-size: 0.82rem;
  color: var(--text-secondary);
}
.loc-item:last-child { border-bottom: none; }

.empty-text {
  font-size: 0.8rem;
  color: var(--text-muted);
  margin-top: 0.5rem;
}
</style>
