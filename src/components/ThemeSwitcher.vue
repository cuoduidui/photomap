<template>
  <div class="theme-switcher" :class="{ inline }">
    <button
      v-for="t in themes"
      :key="t.id"
      class="theme-swatch"
      :class="{ active: current === t.id }"
      :title="t.name"
      @click="$emit('select', t.id)"
    >
      <span class="swatch-dot" :style="{ background: t.preview }" />
      <span class="swatch-name">{{ t.name }}</span>
      <span v-if="current === t.id" class="swatch-check">✓</span>
    </button>
  </div>
</template>

<script setup>
import { THEMES } from "../utils/theme";

defineProps({
  current: { type: String, default: "fresh" },
  inline: { type: Boolean, default: false },
});
defineEmits(["select"]);

const themes = THEMES;
</script>

<style scoped>
.theme-switcher {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.theme-switcher.inline {
  gap: 0.4rem;
}

.theme-swatch {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.4rem 0.7rem;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--rule);
  color: var(--text-secondary);
  font-size: 0.78rem;
  transition: all 0.15s ease;
}
.theme-switcher.inline .theme-swatch {
  padding: 0.3rem;
  gap: 0;
}
.theme-swatch:hover {
  border-color: var(--accent);
  color: var(--text-primary);
  transform: translateY(-1px);
}
.theme-swatch.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--text-primary);
  font-weight: 600;
}

.swatch-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}
.theme-switcher.inline .swatch-name {
  display: none;
}

.swatch-name {
  white-space: nowrap;
}
.swatch-check {
  margin-left: 0.1rem;
  color: var(--accent);
  font-size: 0.7rem;
}
</style>
