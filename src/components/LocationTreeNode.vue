<template>
  <div class="tn-wrap">
    <div class="tn-row" :class="{ active, 'tn-province': node.kind === 'province' }"
      :style="{ paddingLeft: indent + 'px' }" @click="onRowClick">
      <span v-if="hasChildren" class="tree-arrow arrow-btn" :class="{ open: expanded }"
        @click.stop="toggle">{{ expanded ? '▾' : '▸' }}</span>
      <span v-else class="tree-arrow arrow-placeholder"></span>
      <span class="tn-dot" :class="'dot-' + node.kind"></span>
      <span class="tn-name" :title="node.title || node.name">{{ node.name }}</span>
      <span class="tn-count">{{ node.count }}</span>
    </div>
    <div v-if="hasChildren" v-show="expanded" class="tn-children">
      <LocationTreeNode
        v-for="child in node.children"
        :key="child.key"
        :node="child"
        :active-key="activeKey"
        :indent="indent + 12"
        @select="(n) => emit('select', n)" />
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";

defineOptions({ name: "LocationTreeNode" });

const props = defineProps({
  node: { type: Object, required: true },
  activeKey: { type: String, default: "" },
  indent: { type: Number, default: 0 },
});

const emit = defineEmits(["select"]);
const expanded = ref(true);

const hasChildren = computed(() => props.node.children && props.node.children.length > 0);

const active = computed(() => {
  if (!props.activeKey) return false;
  return props.activeKey === props.node.key || props.activeKey.startsWith(props.node.key + "|");
});

function toggle() {
  expanded.value = !expanded.value;
}

function onRowClick() {
  emit("select", props.node);
  if (props.node.kind === "province" && hasChildren.value) {
    toggle();
  }
}
</script>

<style scoped>
.tn-wrap {
  margin-bottom: 2px;
}
.tn-row {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.15s ease;
  user-select: none;
}
.tn-row:hover {
  background: var(--bg-hover);
}
.tn-row.active {
  background: rgba(14, 165, 233, 0.08);
}
.tn-row.active .tn-dot {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}
.tn-row.active .tn-name {
  color: var(--accent);
  font-weight: 600;
}

.tn-province {
  font-weight: 600;
  color: var(--text-primary);
  margin-top: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
}

.tree-arrow {
  font-size: 0.68rem;
  color: var(--text-muted);
  width: 12px;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}
.tree-arrow.open {
  transform: rotate(90deg);
}
.arrow-btn {
  cursor: pointer;
}
.arrow-placeholder {
  visibility: hidden;
}

.tn-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0.55;
  flex-shrink: 0;
}
.dot-province {
  width: 10px;
  height: 10px;
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  opacity: 1;
}
.dot-city {
  background: var(--accent);
  opacity: 0.85;
}
.dot-district {
  width: 7px;
  height: 7px;
  background: var(--accent-2);
  opacity: 0.8;
}
.dot-street {
  width: 6px;
  height: 6px;
  background: #10b981;
  opacity: 0.75;
}
.dot-coord {
  width: 7px;
  height: 7px;
  background: #8b5cf6;
  opacity: 0.85;
}
.dot-address {
  width: 5px;
  height: 5px;
  opacity: 0.4;
}

.tn-name {
  flex: 1;
  min-width: 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tn-province .tn-name {
  font-size: 0.82rem;
}
.tn-count {
  flex-shrink: 0;
  font-size: 0.7rem;
  color: var(--text-muted);
  background: var(--bg-card);
  padding: 2px 7px;
  border-radius: 10px;
}

.tn-children {
  margin-left: 12px;
  padding-left: 6px;
  border-left: 1px solid var(--border);
}
</style>
