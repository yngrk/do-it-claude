<script setup lang="ts">
import { watch, computed } from 'vue'
import { useTaskStore } from '../stores/taskStore'
import { TASK_TAGS } from '../types'
import type { Task } from '../types'

const props = defineProps<{ task: Task | null }>()
const emit = defineEmits<{ close: []; retry: [taskId: string] }>()

const taskStore = useTaskStore()

watch(() => props.task, (task) => {
  if (task) {
    taskStore.loadTaskLogs(task.id)
  }
})

const logs = computed(() => {
  if (!props.task) return []
  return taskStore.taskLogs[props.task.id] || []
})

const liveOutput = computed(() => {
  if (!props.task) return []
  return taskStore.liveLogs[props.task.id] || []
})

const tagInfo = computed(() => {
  if (!props.task?.tag) return null
  return TASK_TAGS.find(t => t.value === props.task!.tag) ?? null
})

function formatDate(date: string | null) {
  if (!date) return '—'
  return new Date(date).toLocaleString()
}
</script>

<template>
  <div v-if="task" class="modal-overlay" @click.self="emit('close')">
    <div class="modal task-detail-modal slide-up">
      <div class="modal-header">
        <h2>Task Detail</h2>
        <button class="btn-icon" @click="emit('close')" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="modal-body">
        <h3>{{ task.title }}</h3>
        <span v-if="tagInfo" class="detail-tag" :style="{ background: tagInfo.color + '18', color: tagInfo.color, borderColor: tagInfo.color + '33' }">
          {{ tagInfo.label }}
        </span>
        <p class="task-description">{{ task.description }}</p>

        <div class="detail-meta">
          <div class="meta-row">
            <span class="detail-label meta-label">Status</span>
            <span :class="`badge badge-${task.status}`">{{ task.status.replace('_', ' ') }}</span>
          </div>
          <div class="meta-row" v-if="task.exit_code !== null">
            <span class="detail-label meta-label">Exit Code</span>
            <span>{{ task.exit_code }}</span>
          </div>
          <div class="meta-row">
            <span class="detail-label meta-label">Created</span>
            <span>{{ formatDate(task.created_at) }}</span>
          </div>
          <div class="meta-row">
            <span class="detail-label meta-label">Started</span>
            <span>{{ formatDate(task.started_at) }}</span>
          </div>
          <div class="meta-row">
            <span class="detail-label meta-label">Completed</span>
            <span>{{ formatDate(task.completed_at) }}</span>
          </div>
        </div>

        <div v-if="task.status === 'in_progress' && liveOutput.length > 0" class="log-section">
          <h4 class="detail-label">Live Output</h4>
          <div class="log-output log-output-mono">
            <div v-for="(line, i) in liveOutput" :key="i" class="log-line log-live">{{ line }}</div>
            <span class="live-cursor"></span>
          </div>
        </div>

        <div v-if="logs.length > 0" class="log-section">
          <h4 class="detail-label">Logs</h4>
          <div class="log-output log-output-mono">
            <div
              v-for="log in logs"
              :key="log.id"
              :class="`log-line log-${log.log_type}`"
            >{{ log.content }}</div>
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button
          v-if="task.status === 'failed'"
          class="btn btn-primary"
          @click="emit('retry', task.id)"
        >Retry</button>
        <button class="btn btn-secondary" @click="emit('close')">Close</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-output-mono {
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.8rem;
  position: relative;
}

.detail-tag {
  display: inline-flex;
  padding: 2px 10px;
  border-radius: 100px;
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  border: 1px solid;
  margin-top: 4px;
}

.detail-label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.7rem;
  font-weight: 600;
}

.live-cursor {
  display: inline-block;
  width: 7px;
  height: 13px;
  background-color: currentColor;
  opacity: 0.7;
  margin-left: 2px;
  vertical-align: text-bottom;
  animation: blink-cursor 1s step-end infinite;
}

@keyframes blink-cursor {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 0; }
}
</style>
