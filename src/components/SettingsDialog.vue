<template>
  <div class="dialog-overlay" @click="emit('close')">
    <div class="dialog-card" @click.stop>
      <div class="dialog-header">
        <h3>设置</h3>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="dialog-body">
        <!-- 高德地图 API -->
        <div class="setting-section">
          <h4>高德地图 API Key</h4>
          <p class="setting-desc">
            用于逆地理编码和地址搜索。前往
            <a href="https://lbs.amap.com/" target="_blank" rel="noopener">高德开放平台</a>
            申请。
          </p>
          <div class="input-row">
            <input type="text" v-model="apiKey" placeholder="输入高德API Key" class="api-input" />
            <button class="save-btn" @click="saveApiKey">保存</button>
          </div>
          <div v-if="saveMsg" class="save-msg" :class="{ error: saveError }">{{ saveMsg }}</div>

          <!-- 批量逆地理编码 -->
          <div class="geocode-section">
            <button class="geocode-btn" @click="doBatchGeocode" 
              :disabled="geocoding || !apiKey.trim()">
              <span v-if="geocoding">
                <span class="btn-spinner"></span>
                逆地理编码中 {{ geocodeDone }}/{{ geocodeTotal }}
              </span>
              <span v-else>📍 批量补全地址信息</span>
            </button>
            <p class="geocode-desc">
              为所有有 GPS 坐标但缺少地址的照片，自动获取省市区和详细地址
            </p>
            <div v-if="geocodeResult" class="geocode-result">
              ✅ 已为 {{ geocodeResult }} 张照片补全地址
            </div>
          </div>
        </div>

        <!-- AI 游记配置 -->
        <div class="setting-section">
          <h4>AI 游记生成</h4>
          <p class="setting-desc">配置 AI 服务，自动为旅行生成游记文章</p>

          <div class="config-group">
            <label class="config-label">AI 服务商</label>
            <select v-model="aiProvider" class="config-select">
              <option value="openai">OpenAI (GPT)</option>
              <option value="deepseek">DeepSeek</option>
              <option value="qwen">通义千问</option>
              <option value="ollama">Ollama (本地)</option>
            </select>
          </div>

          <div class="config-group">
            <label class="config-label">API Key</label>
            <input type="password" v-model="aiApiKey" placeholder="输入 AI API Key"
              class="api-input" />
            <p v-if="aiProvider === 'ollama'" class="config-hint">
              Ollama 无需 API Key，确保本地已运行 ollama 服务
            </p>
          </div>

          <div class="config-group">
            <label class="config-label">模型名称</label>
            <input type="text" v-model="aiModel" :placeholder="modelPlaceholder"
              class="api-input" />
          </div>

          <div class="config-group">
            <label class="config-label">API 地址（可选）</label>
            <input type="text" v-model="aiBaseUrl" placeholder="自定义 API 端点"
              class="api-input" />
          </div>

          <button class="save-btn" @click="saveAiConfig">保存 AI 配置</button>
          <div v-if="aiSaveMsg" class="save-msg" :class="{ error: aiSaveError }">{{ aiSaveMsg }}</div>
        </div>

        <!-- 外观 / 皮肤 -->
        <div class="setting-section">
          <h4>外观 · 皮肤</h4>
          <p class="setting-desc">一键切换整体配色，选择后立即生效并自动保存</p>
          <ThemeSwitcher :current="currentTheme" @select="onSelectTheme" />
        </div>

        <!-- 快捷地点 -->
        <div class="setting-section">
          <h4>快捷地点管理</h4>
          <div class="add-location">
            <input type="text" v-model="newLocName" placeholder="地点名称" />
            <input type="number" v-model="newLocLat" placeholder="纬度" step="0.0001" />
            <input type="number" v-model="newLocLng" placeholder="经度" step="0.0001" />
            <button class="save-btn" @click="addLocation">添加</button>
          </div>
          <div v-if="store.customLocations.length" class="loc-list">
            <div v-for="loc in store.customLocations" :key="loc.id" class="loc-item">
              <span>{{ loc.name }} ({{ loc.latitude.toFixed(4) }}, {{ loc.longitude.toFixed(4) }})</span>
              <button class="del-btn" @click="store.removeCustomLocation(loc.id)">删除</button>
            </div>
          </div>
          <p v-else class="empty-text">暂无快捷地点</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, onUnmounted } from "vue";
import { usePhotoStore } from "../stores/photoStore";
import { getConfig, setConfig, onGeocodeProgress } from "../utils/tauri";
import ThemeSwitcher from "./ThemeSwitcher.vue";
import { loadTheme, setTheme, THEMES } from "../utils/theme";

const emit = defineEmits(["close"]);
const store = usePhotoStore();

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
let unlistenGeocode = null;

async function doBatchGeocode() {
  if (geocoding.value || !apiKey.value.trim()) return;
  geocoding.value = true;
  geocodeDone.value = 0;
  geocodeTotal.value = 0;
  geocodeResult.value = null;
  
  try {
    const updated = await store.runBatchGeocode();
    geocodeResult.value = updated;
  } catch (e) {
    console.error("逆地理编码失败:", e);
  } finally {
    geocoding.value = false;
  }
}

const modelPlaceholder = computed(() => {
  switch (aiProvider.value) {
    case "openai": return "gpt-4o-mini";
    case "deepseek": return "deepseek-chat";
    case "qwen": return "qwen-plus";
    case "ollama": return "qwen2.5:7b";
    default: return "模型名称";
  }
});

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
    saveMsg.value = "API Key 已保存";
  } catch (e) {
    saveMsg.value = "保存失败: " + e;
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
    aiSaveMsg.value = "AI 配置已保存";
  } catch (e) {
    aiSaveMsg.value = "保存失败: " + e;
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
    saveMsg.value = "添加失败: " + e;
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
