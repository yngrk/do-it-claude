<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useTaskStore } from '../stores/taskStore'
import { DEFAULT_TASK_TAGS } from '../types'
import type { Task } from '../types'

const props = defineProps<{ task: Task | null }>()
const emit = defineEmits<{ close: []; retry: [taskId: string]; 'move-to-backlog': [taskId: string] }>()

const taskStore = useTaskStore()

const editTitle = ref('')
const editDescription = ref('')
const editTag = ref<string | null>(null)
const saving = ref(false)
const logsExpanded = ref(false)

const isEditable = computed(() => props.task?.status === 'backlog' || props.task?.status === 'queued')

watch(() => props.task, (task) => {
  logsExpanded.value = false
  if (task) {
    taskStore.loadTaskLogs(task.id)
    editTitle.value = task.title
    editDescription.value = task.description
    editTag.value = task.tag || null
  }
})

function selectTag(value: string) {
  editTag.value = editTag.value === value ? null : value
}

async function save() {
  if (!props.task || !editTitle.value) return
  saving.value = true
  try {
    await taskStore.updateTask(props.task.id, {
      title: editTitle.value,
      description: editDescription.value,
      tag: editTag.value,
    })
  } finally {
    saving.value = false
  }
}

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
  return DEFAULT_TASK_TAGS.find(t => t.value === props.task!.tag) ?? null
})

function formatDate(date: string | null) {
  if (!date) return null
  const d = new Date(date)
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) + ' at ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
}

function formatDuration(start: string | null, end: string | null) {
  if (!start || !end) return null
  const ms = new Date(end).getTime() - new Date(start).getTime()
  if (ms < 1000) return '<1s'
  const secs = Math.floor(ms / 1000)
  if (secs < 60) return `${secs}s`
  const mins = Math.floor(secs / 60)
  const remainSecs = secs % 60
  if (mins < 60) return `${mins}m ${remainSecs}s`
  const hours = Math.floor(mins / 60)
  const remainMins = mins % 60
  return `${hours}h ${remainMins}m`
}

const duration = computed(() => formatDuration(props.task?.started_at ?? null, props.task?.completed_at ?? null))

// Dirty check for save button
const isDirty = computed(() => {
  if (!props.task) return false
  return editTitle.value !== props.task.title
    || editDescription.value !== props.task.description
    || (editTag.value || null) !== (props.task.tag || null)
})
</script>

<template>
  <div v-if="task" class="modal-overlay" @click.self="emit('close')">
    <div class="modal td-modal slide-up">
      <!-- Header -->
      <div class="td-header">
        <div class="td-header-left">
          <span :class="`td-status-dot td-dot-${task.status}`"></span>
          <span class="td-header-title">Task Detail</span>
        </div>
        <button class="btn-icon" @click="emit('close')" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="td-body">
        <!-- Title -->
        <div class="td-section">
          <span class="td-section-label">Title</span>
          <input
            v-if="isEditable"
            v-model="editTitle"
            type="text"
            class="td-field-input"
            placeholder="Task title..."
          />
          <div v-else class="td-prompt-view">{{ task.title }}</div>
        </div>
        <!-- Labels -->
        <div class="td-labels">
          <template v-if="isEditable">
            <button
              v-for="t in DEFAULT_TASK_TAGS"
              :key="t.value"
              class="td-label-chip"
              :class="{ 'td-label-active': editTag === t.value }"
              :style="editTag === t.value
                ? { background: t.color + '30', borderColor: t.color + '66', color: t.color }
                : { borderColor: t.color + '33', color: t.color + 'aa' }"
              @click="selectTag(t.value)"
              type="button"
            >
              <span class="td-label-dot" :style="{ background: t.color }"></span>
              {{ t.label }}
            </button>
          </template>
          <template v-else>
            <span v-if="tagInfo" class="td-label-chip td-label-active"
              :style="{ background: tagInfo.color + '30', borderColor: tagInfo.color + '66', color: tagInfo.color }">
              <span class="td-label-dot" :style="{ background: tagInfo.color }"></span>
              {{ tagInfo.label }}
            </span>
            <span v-else class="td-no-label">No label</span>
          </template>
        </div>

        <!-- Prompt -->
        <div class="td-section">
          <span class="td-section-label">Prompt</span>
          <textarea
            v-if="isEditable"
            v-model="editDescription"
            class="td-prompt-edit"
            placeholder="Write the prompt Claude will execute..."
            rows="6"
          />
          <div v-else class="td-prompt-view">{{ task.description || 'No prompt provided.' }}</div>
        </div>

        <!-- Meta (only for non-editable / executed tasks) -->
        <div v-if="!isEditable" class="td-meta">
          <div class="td-meta-item" v-if="task.exit_code !== null">
            <span class="td-meta-label">Exit code</span>
            <span class="td-meta-value" :class="task.exit_code === 0 ? 'td-meta-ok' : 'td-meta-err'">{{ task.exit_code }}</span>
          </div>
          <div class="td-meta-item" v-if="duration">
            <span class="td-meta-label">Duration</span>
            <span class="td-meta-value">{{ duration }}</span>
          </div>
          <div class="td-meta-item" v-if="formatDate(task.created_at)">
            <span class="td-meta-label">Created</span>
            <span class="td-meta-value">{{ formatDate(task.created_at) }}</span>
          </div>
          <div class="td-meta-item" v-if="formatDate(task.started_at)">
            <span class="td-meta-label">Started</span>
            <span class="td-meta-value">{{ formatDate(task.started_at) }}</span>
          </div>
          <div class="td-meta-item" v-if="formatDate(task.completed_at)">
            <span class="td-meta-label">Completed</span>
            <span class="td-meta-value">{{ formatDate(task.completed_at) }}</span>
          </div>
        </div>

        <!-- Live output -->
        <div v-if="task.status === 'in_progress' && liveOutput.length > 0" class="td-section">
          <div class="td-section-header">
            <span class="td-section-label">
              <span class="td-live-dot"></span>
              Live Output
            </span>
          </div>
          <div class="td-log-box">
            <div v-for="(line, i) in liveOutput" :key="i" class="td-log-line">{{ line }}</div>
            <span class="td-cursor"></span>
          </div>
        </div>

        <!-- Logs -->
        <div v-if="logs.length > 0" class="td-section">
          <button class="td-section-toggle" @click="logsExpanded = !logsExpanded">
            <svg
              width="10" height="10" viewBox="0 0 10 10" fill="none"
              class="td-chevron" :class="{ 'td-chevron-open': logsExpanded }"
            >
              <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Execution Logs
            <span class="td-log-count">{{ logs.length }}</span>
          </button>
          <div v-if="logsExpanded" class="td-log-box">
            <div v-for="log in logs" :key="log.id" :class="`td-log-line td-log-${log.log_type}`">{{ log.content }}</div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="td-footer">
        <div class="td-footer-left">
          <button
            v-if="task.status === 'done' || task.status === 'failed'"
            class="btn btn-secondary btn-sm"
            @click="emit('move-to-backlog', task.id)"
          >Move to Backlog</button>
          <button
            v-if="task.status === 'failed'"
            class="btn btn-secondary btn-sm"
            @click="emit('retry', task.id)"
          >Retry</button>
        </div>
        <button
          v-if="isEditable && isDirty"
          class="btn btn-primary btn-sm"
          :disabled="!editTitle || saving"
          @click="save"
        >{{ saving ? 'Saving...' : 'Save' }}</button>
        <button class="btn btn-secondary btn-sm" @click="emit('close')">Close</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.td-modal {
  width: 560px;
}

/* Header */
.td-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.td-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.td-header-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
}

.td-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.td-dot-backlog     { background: var(--badge-backlog-color); }
.td-dot-queued      { background: var(--badge-queued-color); }
.td-dot-in_progress { background: var(--badge-in_progress-color); animation: pulse-glow 1.6s ease infinite; }
.td-dot-done        { background: var(--badge-done-color); }
.td-dot-failed      { background: var(--badge-failed-color); }

.td-field-input {
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-weight: 500;
  font-family: inherit;
  outline: none;
  width: 100%;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.td-field-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.td-field-input::placeholder {
  color: var(--text-secondary);
}

/* Body */
.td-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Labels */
.td-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.td-label-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 100px;
  border: 1px solid;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s ease;
  background: transparent;
}

.td-label-chip:not(.td-label-active) {
  opacity: 0.7;
}

.td-label-chip:not(.td-label-active):hover {
  opacity: 1;
}

.td-label-active {
  opacity: 1;
}

.td-label-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.td-no-label {
  font-size: 0.75rem;
  color: var(--text-muted);
}

/* Sections */
.td-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.td-section-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.td-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

/* Prompt — edit */
.td-prompt-edit {
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: inherit;
  line-height: 1.65;
  outline: none;
  resize: vertical;
  min-height: 100px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.td-prompt-edit:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.td-prompt-edit::placeholder {
  color: var(--text-secondary);
}

/* Prompt — view */
.td-prompt-view {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
}

/* Meta grid */
.td-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  padding: 12px 14px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
}

.td-meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.td-meta-label {
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.td-meta-value {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.td-meta-ok  { color: var(--success); font-weight: 600; }
.td-meta-err { color: var(--error); font-weight: 600; }

/* Logs */
.td-section-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  cursor: pointer;
  transition: var(--transition);
}

.td-section-toggle:hover {
  color: var(--text-secondary);
}

.td-chevron { transition: transform 0.15s ease; }
.td-chevron-open { transform: rotate(90deg); }

.td-log-count {
  font-size: 0.625rem;
  font-weight: 600;
  color: var(--text-muted);
  background: var(--hover-overlay);
  padding: 0 5px;
  border-radius: 100px;
  line-height: 1.6;
}

.td-log-box {
  background: var(--bg-terminal);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  padding: 10px 14px;
  max-height: 240px;
  overflow-y: auto;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.6875rem;
  line-height: 1.65;
}

.td-log-line {
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-all;
}

.td-log-stderr { color: #f87171; }

.td-live-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success);
  animation: pulse-glow 1.6s ease infinite;
}

.td-cursor {
  display: inline-block;
  width: 5px;
  height: 10px;
  background: var(--text-secondary);
  animation: blink-cursor 1s step-end infinite;
}

@keyframes blink-cursor {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 0; }
}

/* Footer */
.td-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 20px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.td-footer-left {
  display: flex;
  gap: 8px;
  margin-right: auto;
}
</style>
