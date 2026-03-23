<script setup lang="ts">
import { ref } from 'vue'
import TerminalPanel from './TerminalPanel.vue'

defineProps<{ cwd: string | null }>()

const panelHeight = ref(220)
const isDragging = ref(false)

function startResize(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
  const startY = e.clientY
  const startHeight = panelHeight.value

  function onMove(e: MouseEvent) {
    const delta = startY - e.clientY
    panelHeight.value = Math.max(100, Math.min(startHeight + delta, window.innerHeight - 120))
  }

  function onUp() {
    isDragging.value = false
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }

  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}
</script>

<template>
  <div class="bottom-panel" :style="{ height: panelHeight + 'px' }">
    <div class="resize-handle" @mousedown="startResize"></div>
    <div class="panel-content">
      <TerminalPanel v-if="cwd" :cwd="cwd" />
      <div v-else class="panel-empty">Select a project to open a terminal.</div>
    </div>
  </div>
  <div v-if="isDragging" class="drag-overlay"></div>
</template>

<style scoped>
.bottom-panel {
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
  background: var(--bg-terminal);
  flex-shrink: 0;
  position: relative;
}

.resize-handle {
  position: absolute;
  top: -2px;
  left: 0;
  right: 0;
  height: 4px;
  cursor: ns-resize;
  z-index: 10;
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 1px;
  left: 0;
  right: 0;
  height: 1px;
  background: transparent;
  transition: background 0.15s ease;
}

.resize-handle:hover::after {
  background: var(--accent);
}

.drag-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  cursor: ns-resize;
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.panel-empty {
  padding: 12px;
  color: var(--text-muted);
  font-size: 0.75rem;
}
</style>
