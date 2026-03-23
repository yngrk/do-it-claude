<script setup lang="ts">
import type { Task } from '../types'

defineProps<{ task: Task }>()
const emit = defineEmits<{ 'open-detail': [] }>()
</script>

<template>
  <div class="task-card" @click="emit('open-detail')">
    <div class="task-card-top">
      <p class="task-title">{{ task.title }}</p>
      <svg class="chevron-icon" width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M4.5 2.5L8 6L4.5 9.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
    <p v-if="task.description" class="task-prompt">{{ task.description }}</p>
  </div>
</template>

<style scoped>
.task-card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}

.task-title {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
  line-height: 1.4;
}

.task-prompt {
  margin-top: 4px;
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
</style>
