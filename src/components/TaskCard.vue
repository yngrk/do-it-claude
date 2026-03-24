<script setup lang="ts">
import { computed } from 'vue'
import { DEFAULT_TASK_TAGS } from '../types'
import type { Task } from '../types'

const props = defineProps<{ task: Task, deletable?: boolean }>()
const emit = defineEmits<{ 'open-detail': [], 'delete': [] }>()

function onDelete(e: Event) {
  e.stopPropagation()
  emit('delete')
}

const tagInfo = computed(() => {
  if (!props.task.tag) return null
  return DEFAULT_TASK_TAGS.find(t => t.value === props.task.tag) ?? null
})
</script>

<template>
  <div class="task-card" @click="emit('open-detail')">
    <div class="task-card-top">
      <span v-if="tagInfo" class="task-tag" :style="{ background: tagInfo.color + '18', color: tagInfo.color, borderColor: tagInfo.color + '33' }">
        {{ tagInfo.label }}
      </span>
      <p class="task-title">{{ task.title }}</p>
      <button v-if="deletable" class="delete-btn" @click="onDelete" title="Delete task">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
      <svg v-else class="chevron-icon" width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M4.5 2.5L8 6L4.5 9.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
    <div v-if="task.description" class="task-card-bottom">
      <p class="task-prompt">{{ task.description }}</p>
    </div>
  </div>
</template>

<style scoped>
.task-card-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.task-title {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
  line-height: 1.4;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-card-bottom {
  margin-top: 4px;
}

.task-tag {
  display: inline-flex;
  flex-shrink: 0;
  padding: 1px 8px;
  border-radius: 100px;
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  border: 1px solid;
}

.task-prompt {
  font-size: 0.75rem;
  color: var(--text-muted);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.chevron-icon {
  color: var(--text-muted);
  opacity: 0.4;
  flex-shrink: 0;
  margin-top: 2px;
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.task-card:hover .chevron-icon {
  opacity: 1;
  transform: translateX(2px);
}

.delete-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.task-card:hover .delete-btn {
  opacity: 0.6;
}

.delete-btn:hover {
  opacity: 1 !important;
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

</style>
