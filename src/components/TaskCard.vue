<script setup lang="ts">
import type { Task } from '../types'

defineProps<{ task: Task }>()
const emit = defineEmits<{ 'open-detail': [] }>()

function formatDate(date: string | null) {
  if (!date) return null
  return new Date(date).toLocaleString()
}
</script>

<template>
  <div class="task-card" @click="emit('open-detail')">
    <div class="task-card-header">
      <span :class="`badge badge-${task.status}`">{{ task.status.replace('_', ' ') }}</span>
    </div>
    <p class="task-title">{{ task.title }}</p>
    <div class="task-meta">
      <span v-if="task.started_at" class="task-time">Started: {{ formatDate(task.started_at) }}</span>
      <span v-if="task.completed_at" class="task-time">Done: {{ formatDate(task.completed_at) }}</span>
    </div>
  </div>
</template>
