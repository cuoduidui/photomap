<template>
  <div class="tag-panel">
    <div class="tag-header">
      <span class="tag-title">{{ $t("tag.title") }}</span>
      <div class="header-actions">
        <button class="analyze-btn" @click="onAnalyzeFaces" :disabled="analyzing || store.photos.length === 0">
          {{ analyzing ? $t('tag.analyzing') : $t('tag.analyzeFaces') }}
        </button>
        <button class="add-btn" @click="showAdd = true" :title="$t('tag.newTag')">+</button>
      </div>
    </div>

    <!-- 分析进度 -->
    <div v-if="analyzing" class="analyze-progress">
      <div class="progress-info">
        <span>{{ $t("tag.analyzingFaces") }}</span>
        <span class="progress-numbers">{{ analyzeDone }} / {{ analyzeTotal }}</span>
      </div>
      <div class="analyze-bar-track">
        <div class="analyze-bar-fill" :style="{ width: analyzePercent + '%' }"></div>
      </div>
      <button class="analyze-cancel-btn" @click="cancelAnalyze">{{ $t("common.cancel") }}</button>
    </div>

    <div v-if="showAdd" class="add-tag-form">
      <input v-model="newTagName" class="tag-input" :placeholder="$t('tag.tagNamePlaceholder')" @keyup.enter="onCreateTag" />
      <div class="tag-type-row">
        <label class="type-radio">
          <input type="radio" v-model="newTagType" value="person" />
          <span>👤 {{ $t("tag.person") }}</span>
        </label>
        <label class="type-radio">
          <input type="radio" v-model="newTagType" value="custom" />
          <span>🏷️ {{ $t("tag.label") }}</span>
        </label>
      </div>
      <div class="color-row">
        <div v-for="c in colorOptions" :key="c" class="color-dot"
          :class="{ active: newTagColor === c }"
          :style="{ background: c }"
          @click="newTagColor = c"></div>
      </div>
      <div class="form-actions">
        <button class="cancel-btn" @click="showAdd = false">{{ $t("common.cancel") }}</button>
        <button class="confirm-btn" @click="onCreateTag">{{ $t("tag.create") }}</button>
      </div>
    </div>

    <div class="tag-content">
      <!-- 人物标签 -->
      <div v-if="personTags.length > 0" class="tag-group">
        <div class="group-title">
          <span>👤 {{ $t("tag.person") }}</span>
          <span class="group-count">{{ personTags.length }}</span>
        </div>
        <div class="tag-list">
          <div v-for="tag in personTags" :key="tag.id" class="tag-item person-tag"
            :class="{ active: isActive(tag.id) }"
            @click="onToggleTag(tag.id)">
            <div class="tag-avatar" v-if="faceThumbs.get(tag.id)">
              <img :src="faceThumbs.get(tag.id)" alt="" />
            </div>
            <span v-else class="tag-color" :style="{ background: tag.color }"></span>
            <div class="tag-info">
              <span v-if="editingId !== tag.id" class="tag-name" @click.stop="startEditName(tag)">
                {{ tag.name }}
                <span class="edit-hint">✎</span>
              </span>
              <input v-else class="tag-name-input"
                v-model="editingName"
                @blur="saveEditName(tag)"
                @keyup.enter="saveEditName(tag)"
                @keyup.escape="cancelEdit"
                ref="nameInput" />
              <span v-if="tag.description" class="tag-desc-inline">{{ tag.description }}</span>
            </div>
            <span class="tag-count">{{ tag.count }}</span>
            <button class="tag-edit-btn" @click.stop="openEditModal(tag)" :title="$t('tag.edit')">✎</button>
            <button class="tag-delete" @click.stop="onDeleteTag(tag.id)" :title="$t('common.delete')">×</button>
          </div>
        </div>
      </div>

      <!-- 自定义标签 -->
      <div v-if="customTags.length > 0" class="tag-group">
        <div class="group-title">
          <span>🏷️ {{ $t("tag.label") }}</span>
          <span class="group-count">{{ customTags.length }}</span>
        </div>
        <div class="tag-list">
          <div v-for="tag in customTags" :key="tag.id" class="tag-item"
            :class="{ active: isActive(tag.id) }"
            @click="onToggleTag(tag.id)">
            <span class="tag-color" :style="{ background: tag.color }"></span>
            <div class="tag-info">
              <span v-if="editingId !== tag.id" class="tag-name" @click.stop="startEditName(tag)">
                {{ tag.name }}
                <span class="edit-hint">✎</span>
              </span>
              <input v-else class="tag-name-input"
                v-model="editingName"
                @blur="saveEditName(tag)"
                @keyup.enter="saveEditName(tag)"
                @keyup.escape="cancelEdit"
                ref="nameInput" />
            </div>
            <span class="tag-count">{{ tag.count }}</span>
            <button class="tag-edit-btn" @click.stop="openEditModal(tag)" :title="$t('tag.edit')">✎</button>
            <button class="tag-delete" @click.stop="onDeleteTag(tag.id)" :title="$t('common.delete')">×</button>
          </div>
        </div>
      </div>

      <div v-if="store.tags.length === 0 && !analyzing" class="empty-state">
        <div class="empty-icon">🏷️</div>
        <div class="empty-text">{{ $t("tag.noTags") }}</div>
        <div class="empty-hint">{{ $t("tag.noTagsHint") }}</div>
      </div>

      <!-- 批量打标签 -->
      <div v-if="store.selectedPhotoIds.length > 0" class="batch-section">
        <div class="batch-title">
          {{ $t("tag.selectedCount", { n: store.selectedPhotoIds.length }) }}
        </div>
        <div class="batch-tags">
          <div v-for="tag in store.tags" :key="tag.id" class="batch-tag"
            @click="onBatchAddTag(tag.id)">
            + {{ tag.name }}
          </div>
        </div>
      </div>
    </div>

    <!-- 编辑标签弹窗 -->
    <div v-if="editModalTag" class="edit-modal-overlay" @click="editModalTag = null">
      <div class="edit-modal" @click.stop>
        <div class="edit-modal-header">
          <span class="edit-modal-title">{{ $t("tag.editTagTitle", { name: editModalTag.name }) }}</span>
          <button class="edit-modal-close" @click="editModalTag = null">✕</button>
        </div>
        <div class="edit-modal-body">
          <div v-if="editModalTag.face_thumb || editModalTag.tag_type === 'person'" class="edit-avatar-section">
            <div class="edit-avatar" v-if="editModalFaceThumb || faceThumbs.get(editModalTag.id)">
              <img :src="editModalFaceThumb || faceThumbs.get(editModalTag.id)" alt="" />
            </div>
            <div v-else class="edit-avatar-placeholder" :style="{ background: editModalTag.color }">
              <span>{{ editModalTag.name.charAt(0) }}</span>
            </div>
            <div class="avatar-actions">
              <button class="avatar-btn" @click="showPhotoPicker = !showPhotoPicker">
                📷 {{ $t("tag.fromAlbum") }}
              </button>
              <button class="avatar-btn" @click="onSelectLocalPhoto">
                📁 {{ $t("tag.fromLocal") }}
              </button>
            </div>
          </div>

          <!-- 相册照片选择器 -->
          <div v-if="showPhotoPicker" class="photo-picker">
            <div class="photo-picker-grid">
              <div v-for="photo in pickerPhotos" :key="photo.id" class="picker-photo"
                @click="onSelectAlbumPhoto(photo)">
                <img v-if="photo.thumbnail_b64" :src="photo.thumbnail_b64" alt="" />
                <div v-else class="picker-placeholder">📷</div>
              </div>
            </div>
            <div class="picker-more" v-if="pickerPhotos.length === 0">{{ $t("tag.loadingPhotos") }}</div>
          </div>

          <div class="form-item">
            <label class="form-label">{{ $t("tag.name") }}</label>
            <input v-model="editModalName" class="form-input" :placeholder="$t('tag.tagNamePlaceholder')" />
          </div>
          <div class="form-item">
            <label class="form-label">{{ $t("tag.description") }}</label>
            <textarea v-model="editModalDesc" class="form-textarea"
              :placeholder="$t('tag.descPlaceholder')" rows="3"></textarea>
          </div>
          <div class="form-item">
            <label class="form-label">{{ $t("tag.photoList", { n: tagPhotos.length }) }}</label>
            <div v-if="tagPhotos.length > 0" class="tag-photo-grid">
              <div v-for="(photo, idx) in tagPhotos" :key="photo.id" class="tag-photo-item"
                @click="openTagPhotoViewer(idx)">
                <img v-if="photo._thumb" :src="photo._thumb" alt="" />
                <div v-else class="tag-photo-placeholder">📷</div>
              </div>
            </div>
            <div v-else class="no-photos">{{ $t("tag.noPhotos") }}</div>
          </div>
        </div>
        <div class="edit-modal-footer">
          <button class="cancel-btn" @click="editModalTag = null">{{ $t("common.cancel") }}</button>
          <button class="confirm-btn" @click="saveEditModal">{{ $t("common.save") }}</button>
        </div>
      </div>
    </div>

    <!-- 照片查看器 -->
    <PhotoViewer
      v-if="viewerVisible"
      :photo="tagPhotos[viewerIndex]"
      :photo-list="tagPhotos"
      @close="viewerVisible = false" />
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { usePhotoStore } from "../stores/photoStore";
import { getImageBase64 } from "../utils/tauri";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import PhotoViewer from "./PhotoViewer.vue";

const store = usePhotoStore();
const { t } = useI18n();

const showAdd = ref(false);
const newTagName = ref("");
const newTagType = ref("person");
const newTagColor = ref("#10ac84");

const colorOptions = [
  "#10ac84", "#5ba882", "#d47070", "#8b5cf6",
  "#ec4899", "#14b8a6", "#f97316", "#6366f1",
];

const editModalFaceThumb = ref(null);
const showPhotoPicker = ref(false);
const pickerPhotos = ref([]);
const pickerPage = ref(0);
const pickerPageSize = 30;

const personTags = computed(() =>
  store.tags.filter((t) => t.tag_type === "person")
);

const customTags = computed(() =>
  store.tags.filter((t) => t.tag_type === "custom")
);

const analyzing = ref(false);
const analyzeDone = ref(0);
const analyzeTotal = ref(0);
const analyzePercent = computed(() => {
  if (!analyzeTotal.value) return 0;
  return Math.min(100, Math.round((analyzeDone.value / analyzeTotal.value) * 100));
});

const faceThumbs = ref(new Map());

const editingId = ref(null);
const editingName = ref("");

const editModalTag = ref(null);
const editModalName = ref("");
const editModalDesc = ref("");

const tagPhotos = ref([]);
const viewerVisible = ref(false);
const viewerIndex = ref(0);
let editModalFaceThumbPath = null;

function isActive(tagId) {
  return store.filter.tagIds && store.filter.tagIds.includes(tagId);
}

function onToggleTag(tagId) {
  store.toggleTagFilter(tagId);
}

async function onCreateTag() {
  if (!newTagName.value.trim()) return;
  try {
    await store.createTag(newTagName.value.trim(), newTagType.value, newTagColor.value);
    newTagName.value = "";
    showAdd.value = false;
  } catch (e) {
    alert(t("tag.createFailed", { error: e }));
  }
}

async function onDeleteTag(id) {
  if (!confirm(t("tag.deleteConfirm"))) return;
  try {
    await store.deleteTag(id);
  } catch (e) {
    console.warn("删除失败:", e);
  }
}

async function onBatchAddTag(tagId) {
  if (store.selectedPhotoIds.length === 0) return;
  try {
    await store.addTagToPhotos(store.selectedPhotoIds, tagId);
  } catch (e) {
    console.warn("批量打标签失败:", e);
  }
}

async function onAnalyzeFaces() {
  if (analyzing.value) return;
  analyzing.value = true;
  analyzeDone.value = 0;
  analyzeTotal.value = store.photos.length;
  try {
    const result = await store.analyzeFaces((done, total) => {
      analyzeDone.value = done;
      analyzeTotal.value = total;
    });
    if (result.persons_found > 0) {
      console.log(`分析完成：发现 ${result.persons_found} 个人物，共 ${result.total_faces} 张含人脸照片`);
    } else {
      console.log("未发现重复出现的人物");
    }
    await loadFaceThumbs();
  } catch (e) {
    console.error("人脸分析失败:", e);
    alert(t("tag.analyzeFailed", { error: e }));
  } finally {
    analyzing.value = false;
  }
}

async function cancelAnalyze() {
  try {
    const { cancelLongTask } = await import("../utils/tauri");
    await cancelLongTask();
  } catch (e) {
    console.warn("取消失败:", e);
  }
}

function startEditName(tag) {
  editingId.value = tag.id;
  editingName.value = tag.name;
}

async function saveEditName(tag) {
  if (editingId.value !== tag.id) return;
  const newName = editingName.value.trim();
  if (newName && newName !== tag.name) {
    try {
      await store.updateTag(tag.id, newName, tag.description || null, null);
    } catch (e) {
      console.error("更新名称失败:", e);
    }
  }
  editingId.value = null;
  editingName.value = "";
}

function cancelEdit() {
  editingId.value = null;
  editingName.value = "";
}

function openEditModal(tag) {
  editModalTag.value = tag;
  editModalName.value = tag.name;
  editModalDesc.value = tag.description || "";
  editModalFaceThumb.value = null;
  editModalFaceThumbPath = null;
  showPhotoPicker.value = false;
  pickerPhotos.value = [];
  pickerPage.value = 0;
  tagPhotos.value = [];
  loadTagPhotos(tag.id);
}

async function loadTagPhotos(tagId) {
  try {
    const photos = await store.getTagPhotos(tagId);
    tagPhotos.value = photos;
    // 加载缩略图
    for (const photo of tagPhotos.value) {
      if (photo.thumbnail_path) {
        try {
          const b64 = await getImageBase64(photo.thumbnail_path);
          if (b64) {
            photo._thumb = b64;
          }
        } catch {}
      }
    }
  } catch (e) {
    console.error("加载标签照片失败:", e);
  }
}

function openTagPhotoViewer(idx) {
  viewerIndex.value = idx;
  viewerVisible.value = true;
}

async function saveEditModal() {
  if (!editModalTag.value) return;
  try {
    const faceThumbPath = editModalFaceThumbPath || null;
    await store.updateTag(
      editModalTag.value.id,
      editModalName.value.trim() || null,
      editModalDesc.value.trim() || null,
      faceThumbPath
    );
    if (faceThumbPath) {
      const b64 = await getImageBase64(faceThumbPath);
      if (b64) {
        faceThumbs.value.set(editModalTag.value.id, b64);
        faceThumbs.value = new Map(faceThumbs.value);
      }
    }
    editModalTag.value = null;
  } catch (e) {
    console.error("更新标签失败:", e);
    alert(t("tag.updateFailed", { error: e }));
  }
}

async function onSelectLocalPhoto() {
  const selected = await openDialog({
    title: t("tag.selectAvatarTitle"),
    multiple: false,
    filters: [{
      name: t("app.imageFiles"),
      extensions: ["jpg", "jpeg", "png", "webp", "bmp"],
    }],
  });
  if (!selected) return;

  try {
    const b64 = await getImageBase64(selected);
    if (b64) {
      editModalFaceThumb.value = b64;
      editModalFaceThumbPath = selected;
    }
  } catch (e) {
    console.error("加载照片失败:", e);
  }
}

async function loadPickerPhotos() {
  const start = pickerPage.value * pickerPageSize;
  const end = start + pickerPageSize;
  const batch = store.photos.slice(start, end);

  for (const photo of batch) {
    if (photo.thumbnail_path) {
      try {
        const b64 = await getImageBase64(photo.thumbnail_path);
        if (b64 && !pickerPhotos.value.find(p => p.id === photo.id)) {
          pickerPhotos.value.push({ ...photo, thumbnail_b64: b64 });
        }
      } catch {}
    }
  }
}

watch(showPhotoPicker, (val) => {
  if (val && pickerPhotos.value.length === 0) {
    loadPickerPhotos();
  }
});

async function onSelectAlbumPhoto(photo) {
  try {
    const b64 = await getImageBase64(photo.file_path);
    if (b64) {
      editModalFaceThumb.value = b64;
      editModalFaceThumbPath = photo.file_path;
    }
    showPhotoPicker.value = false;
  } catch (e) {
    console.error("加载照片失败:", e);
  }
}

async function loadFaceThumbs() {
  for (const tag of store.tags) {
    if (tag.face_thumb && !faceThumbs.value.has(tag.id)) {
      try {
        const b64 = await getImageBase64(tag.face_thumb);
        if (b64) {
          faceThumbs.value.set(tag.id, b64);
          faceThumbs.value = new Map(faceThumbs.value);
        }
      } catch {}
    }
  }
}

watch(() => store.tags, () => {
  loadFaceThumbs();
}, { immediate: true });
</script>

<style scoped>
.tag-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.tag-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
}
.tag-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.analyze-btn {
  font-size: 0.72rem;
  color: #fff;
  padding: 4px 10px;
  border-radius: 10px;
  background: var(--accent, #10ac84);
  box-shadow: 0 2px 8px rgba(16, 172, 132, 0.2);
  font-weight: 500;
}
.analyze-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 3px 10px rgba(16, 172, 132, 0.3);
}
.analyze-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.analyze-cancel-btn {
  margin-top: 6px;
  width: 100%;
  padding: 6px;
  border-radius: 6px;
  font-size: 0.7rem;
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}
.analyze-cancel-btn:hover {
  background: rgba(248, 113, 113, 0.2);
}
.add-btn {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--accent, #10ac84);
  color: #fff;
  font-size: 1rem;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 8px rgba(16, 172, 132, 0.2);
}
.add-btn:hover {
  transform: scale(1.1);
}

.analyze-progress {
  margin: 0 14px 8px;
  padding: 10px 12px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
}
.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: 0.72rem;
  color: var(--text-secondary);
  margin-bottom: 6px;
}
.progress-numbers {
  color: var(--accent);
  font-weight: 600;
}
.analyze-bar-track {
  width: 100%;
  height: 4px;
  background: var(--border);
  border-radius: 2px;
  overflow: hidden;
}
.analyze-bar-fill {
  height: 100%;
  background: var(--accent, #10ac84);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.add-tag-form {
  margin: 0 14px 12px;
  padding: 12px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
}

.tag-input {
  width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel-solid);
  color: var(--text-primary);
  font-size: 0.8rem;
  margin-bottom: 10px;
  box-sizing: border-box;
}
.tag-input:focus {
  border-color: var(--accent);
  outline: none;
}

.tag-type-row {
  display: flex;
  gap: 12px;
  margin-bottom: 10px;
}
.type-radio {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.78rem;
  color: var(--text-secondary);
  cursor: pointer;
}
.type-radio input {
  accent-color: var(--accent);
}

.color-row {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
}
.color-dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  cursor: pointer;
  transition: transform 0.15s ease;
  border: 2px solid transparent;
}
.color-dot:hover {
  transform: scale(1.15);
}
.color-dot.active {
  border-color: var(--text-primary);
  transform: scale(1.15);
}

.form-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.cancel-btn {
  padding: 4px 12px;
  border-radius: var(--radius-sm);
  font-size: 0.78rem;
  color: var(--text-secondary);
  background: var(--bg-hover);
}
.cancel-btn:hover {
  background: var(--border);
}
.confirm-btn {
  padding: 4px 12px;
  border-radius: var(--radius-sm);
  font-size: 0.78rem;
  color: #fff;
  background: var(--accent, #10ac84);
}
.confirm-btn:hover {
  opacity: 0.9;
}

.tag-content {
  flex: 1;
  overflow-y: auto;
  padding: 0 14px 12px;
}

.tag-group {
  margin-bottom: 16px;
}

.group-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
}
.group-count {
  margin-left: auto;
  font-size: 0.7rem;
  color: var(--text-muted);
  font-weight: 400;
}

.tag-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.tag-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
}
.tag-item:hover {
  background: var(--bg-hover);
}
.tag-item.active {
  background: rgba(16, 172, 132, 0.08);
}
.tag-item.active .tag-name {
  color: var(--accent);
  font-weight: 600;
}

.tag-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  border: 1px solid var(--border);
}
.tag-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.tag-color {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.tag-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.tag-name {
  font-size: 0.82rem;
  color: var(--text-secondary);
  cursor: text;
  display: flex;
  align-items: center;
  gap: 4px;
}
.edit-hint {
  font-size: 0.65rem;
  opacity: 0;
  transition: opacity 0.15s ease;
}
.tag-name:hover .edit-hint {
  opacity: 0.6;
}

.tag-name-input {
  width: 100%;
  padding: 2px 6px;
  border: 1px solid var(--accent);
  border-radius: 4px;
  background: var(--bg-panel-solid);
  color: var(--text-primary);
  font-size: 0.82rem;
  outline: none;
  box-sizing: border-box;
}

.tag-desc-inline {
  font-size: 0.68rem;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag-count {
  font-size: 0.72rem;
  color: var(--text-muted);
  background: var(--bg-card);
  padding: 2px 7px;
  border-radius: 10px;
  flex-shrink: 0;
}

.tag-edit-btn {
  opacity: 0;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: rgba(16, 172, 132, 0.1);
  color: var(--accent-2);
  font-size: 0.7rem;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.15s ease;
  flex-shrink: 0;
}
.tag-item:hover .tag-edit-btn {
  opacity: 1;
}
.tag-edit-btn:hover {
  background: rgba(16, 172, 132, 0.2);
}

.tag-delete {
  opacity: 0;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: rgba(239, 68, 68, 0.1);
  color: var(--error);
  font-size: 0.9rem;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.15s ease;
  flex-shrink: 0;
}
.tag-item:hover .tag-delete {
  opacity: 1;
}
.tag-delete:hover {
  background: rgba(239, 68, 68, 0.2);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 30px 20px;
  color: var(--text-muted);
}
.empty-icon { font-size: 1.8rem; margin-bottom: 8px; opacity: 0.5; }
.empty-text { font-size: 0.82rem; color: var(--text-secondary); margin-bottom: 4px; }
.empty-hint { font-size: 0.72rem; color: var(--text-muted); text-align: center; }

.batch-section {
  margin-top: 12px;
  padding: 10px;
  background: rgba(16, 172, 132, 0.06);
  border-radius: var(--radius-sm);
  border: 1px solid rgba(16, 172, 132, 0.15);
}
.batch-title {
  font-size: 0.75rem;
  color: var(--accent-2);
  font-weight: 600;
  margin-bottom: 8px;
}
.batch-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.batch-tag {
  padding: 3px 8px;
  font-size: 0.7rem;
  background: var(--bg-panel-solid);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: pointer;
  color: var(--text-secondary);
  transition: all 0.15s ease;
}
.batch-tag:hover {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}

/* 编辑标签弹窗 */
.edit-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}
.edit-modal {
  width: 90%;
  max-width: 480px;
  max-height: 85vh;
  background: var(--bg-panel-solid);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.edit-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.edit-modal-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}
.edit-modal-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-secondary);
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  justify-content: center;
}
.edit-modal-close:hover {
  background: var(--bg-hover);
}

.edit-modal-body {
  padding: 16px;
  overflow-y: auto;
  flex: 1;
}
.edit-avatar-section {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}
.edit-avatar {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  overflow: hidden;
  border: 3px solid var(--border);
}
.edit-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.edit-avatar-placeholder {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 1.8rem;
  font-weight: 700;
}

.avatar-actions {
  display: flex;
  gap: 6px;
  margin-top: 10px;
}
.avatar-btn {
  font-size: 0.7rem;
  padding: 4px 10px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
}
.avatar-btn:hover {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}

.photo-picker {
  margin: 12px 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 8px;
  background: var(--bg-card);
}
.photo-picker-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 6px;
  max-height: 200px;
  overflow-y: auto;
}
.picker-photo {
  aspect-ratio: 1;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  border: 2px solid transparent;
  transition: border-color 0.15s ease;
}
.picker-photo:hover {
  border-color: var(--accent);
}
.picker-photo img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.picker-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  font-size: 1.2rem;
}
.picker-more {
  text-align: center;
  font-size: 0.75rem;
  color: var(--text-muted);
  padding: 10px;
}
.form-item {
  margin-bottom: 14px;
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
  box-sizing: border-box;
}
.form-input:focus {
  border-color: var(--accent);
}
.form-textarea {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 0.8rem;
  color: var(--text-primary);
  background: var(--bg-card);
  outline: none;
  resize: vertical;
  font-family: inherit;
  box-sizing: border-box;
}
.form-textarea:focus {
  border-color: var(--accent);
}
.form-value {
  font-size: 0.85rem;
  color: var(--text-primary);
  font-weight: 600;
}

.tag-photo-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 6px;
  margin-top: 6px;
  max-height: 180px;
  overflow-y: auto;
  padding: 4px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
}
.tag-photo-item {
  aspect-ratio: 1;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.15s ease;
}
.tag-photo-item:hover {
  border-color: var(--accent);
  transform: scale(1.05);
}
.tag-photo-item img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.tag-photo-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  font-size: 1.2rem;
}
.no-photos {
  font-size: 0.75rem;
  color: var(--text-muted);
  text-align: center;
  padding: 20px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  margin-top: 6px;
}

.edit-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
}
</style>
