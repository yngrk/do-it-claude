<script setup lang="ts">
import { onMounted, ref, computed, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useTaskStore } from '../stores/taskStore'
import { useProjectStore } from '../stores/projectStore'
import KanbanColumn from '../components/KanbanColumn.vue'
import TaskDetail from '../components/TaskDetail.vue'
import AddTaskDialog from '../components/AddTaskDialog.vue'
import type { Task } from '../types'

const route = useRoute()
const taskStore = useTaskStore()
const projectStore = useProjectStore()
const projectId = computed(() => route.params.id as string)
const showAddDialog = ref(false)
const selectedTask = ref<Task | null>(null)
const queueRunning = ref(false)
const historyOpen = ref(false)
const outputEl = ref<HTMLElement | null>(null)
const isGit = ref<boolean | null>(null)
const initingGit = ref(false)
const outputMaximized = ref(false)
const showProjectSettings = ref(false)
const projectEffort = ref('high')
const projectMaxTurns = ref<number | null>(null)

const activeProject = computed(() =>
  projectStore.projects.find(p => p.id === projectId.value) ?? null
)

async function checkGitStatus() {
  if (!activeProject.value) return
  isGit.value = await invoke<boolean>('check_git', { path: activeProject.value.path })
}

async function initGit() {
  if (!activeProject.value) return
  initingGit.value = true
  try {
    await invoke('init_git', { path: activeProject.value.path })
    isGit.value = true
  } finally {
    initingGit.value = false
  }
}

function loadProject() {
  taskStore.loadTasks(projectId.value)
  checkGitStatus()
  if (activeProject.value) {
    taskStore.loadProjectStats(activeProject.value.path)
  }
  queueRunning.value = false
  outputMaximized.value = false
}

onMounted(loadProject)

watch(projectId, loadProject)

const backlogTasks = computed(() =>
  taskStore.tasks.filter(t => t.status === 'backlog').sort((a, b) => a.sort_order - b.sort_order)
)
const queuedTasks = computed(() =>
  taskStore.tasks.filter(t => t.status === 'queued').sort((a, b) => a.sort_order - b.sort_order)
)
const runningTask = computed(() =>
  taskStore.tasks.find(t => t.status === 'in_progress') ?? null
)
const doneTasks = computed(() =>
  taskStore.tasks.filter(t => t.status === 'done').sort((a, b) => a.sort_order - b.sort_order)
)
const failedTasks = computed(() =>
  taskStore.tasks.filter(t => t.status === 'failed').sort((a, b) => a.sort_order - b.sort_order)
)
const historyCount = computed(() => doneTasks.value.length + failedTasks.value.length)

const liveOutput = computed(() => {
  if (!runningTask.value) return []
  return taskStore.liveLogs[runningTask.value.id] || []
})

// Auto-scroll output
watch(() => liveOutput.value.length, async () => {
  if (outputEl.value) {
    await nextTick()
    outputEl.value.scrollTop = outputEl.value.scrollHeight
  }
})

async function handleTaskMoved(taskId: string, newStatus: Task['status'], newSortOrder: number) {
  await taskStore.moveTask(taskId, newStatus, newSortOrder)
}

async function cancelTask() {
  if (!runningTask.value) return
  const taskId = runningTask.value.id
  await taskStore.cancelAndRevert(projectId.value)
  queueRunning.value = false
  await taskStore.moveTask(taskId, 'queued', 0)
}

async function toggleQueue() {
  if (queueRunning.value) {
    // Pause: let current task finish, don't pick up next
    await taskStore.pauseQueue(projectId.value)
    queueRunning.value = false
  } else {
    await taskStore.startQueue(projectId.value)
    queueRunning.value = true
  }
}

function openDetail(task: Task) {
  selectedTask.value = task
}

async function retryTask(taskId: string) {
  await handleTaskMoved(taskId, 'queued', 0)
}

function formatTokens(n: number) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'k'
  return n.toString()
}

function formatDate(date: string | null) {
  if (!date) return ''
  return new Date(date).toLocaleString()
}
</script>

<template>
  <div class="project-board">
    <div v-if="taskStore.loading" class="loading">Loading tasks...</div>

    <template v-else>
      <!-- Git warning -->
      <div v-if="isGit === false" class="git-banner">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="git-banner-icon">
          <path d="M7 1L1 13H13L7 1Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
          <path d="M7 5.5V8.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <circle cx="7" cy="10.5" r="0.5" fill="currentColor"/>
        </svg>
        <span class="git-banner-text">
          Not a git repository. Cancelling tasks won't be able to revert changes.
        </span>
        <button class="git-init-btn" @click="initGit" :disabled="initingGit">
          {{ initingGit ? 'Initializing...' : 'Initialize git' }}
        </button>
      </div>

      <!-- Top row: stats + running task -->
      <div class="top-row">
        <!-- Stats panel -->
        <div class="stats-panel">
          <div class="stats-header">
            <span>Stats</span>
            <button class="stats-cog" @click="showProjectSettings = !showProjectSettings" title="Project settings">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M6 7.5C6.82843 7.5 7.5 6.82843 7.5 6C7.5 5.17157 6.82843 4.5 6 4.5C5.17157 4.5 4.5 5.17157 4.5 6C4.5 6.82843 5.17157 7.5 6 7.5Z" stroke="currentColor" stroke-width="1"/>
                <path d="M9.79 7.6C9.71 7.76 9.71 7.96 9.81 8.12L10.07 8.54C10.18 8.74 10.14 8.99 9.97 9.13L9.13 9.82C8.97 9.95 8.74 9.95 8.58 9.83L8.15 9.54C7.99 9.44 7.81 9.41 7.64 9.48C7.47 9.55 7.3 9.61 7.12 9.65C6.95 9.7 6.81 9.84 6.78 10.03L6.7 10.53C6.66 10.75 6.47 10.92 6.24 10.92H5.17C4.94 10.92 4.75 10.75 4.71 10.53L4.63 10.03C4.6 9.84 4.46 9.7 4.29 9.65C4.11 9.61 3.94 9.55 3.77 9.48C3.6 9.41 3.41 9.44 3.26 9.54L2.83 9.83C2.67 9.95 2.44 9.95 2.28 9.82L1.44 9.13C1.27 9 1.22 8.74 1.34 8.54L1.6 8.12C1.7 7.96 1.7 7.76 1.62 7.6C1.56 7.43 1.51 7.26 1.47 7.08C1.43 6.9 1.3 6.76 1.12 6.72L0.62 6.63C0.39 6.59 0.23 6.4 0.23 6.17V5.17C0.23 4.94 0.39 4.75 0.62 4.71L1.12 4.62C1.3 4.59 1.43 4.45 1.47 4.27C1.51 4.1 1.56 3.93 1.62 3.77C1.7 3.59 1.7 3.4 1.6 3.24L1.34 2.81C1.22 2.62 1.27 2.36 1.44 2.23L2.28 1.54C2.44 1.41 2.67 1.41 2.83 1.53L3.26 1.82C3.41 1.92 3.6 1.94 3.77 1.88C3.94 1.81 4.11 1.75 4.29 1.7C4.46 1.66 4.6 1.52 4.63 1.33L4.71 0.83C4.75 0.6 4.94 0.44 5.17 0.44H6.24C6.47 0.44 6.66 0.6 6.7 0.83L6.78 1.33C6.81 1.52 6.95 1.66 7.12 1.7C7.3 1.75 7.47 1.81 7.64 1.88C7.81 1.94 7.99 1.92 8.15 1.82L8.58 1.53C8.74 1.41 8.97 1.41 9.13 1.54L9.97 2.23C10.14 2.36 10.18 2.62 10.07 2.81L9.81 3.24C9.71 3.4 9.71 3.59 9.79 3.77C9.85 3.93 9.91 4.1 9.95 4.27C9.99 4.45 10.12 4.59 10.3 4.62L10.8 4.71C11.03 4.75 11.19 4.94 11.19 5.17V6.17C11.19 6.4 11.03 6.59 10.8 6.63L10.3 6.72C10.12 6.76 9.99 6.9 9.95 7.08C9.91 7.26 9.85 7.43 9.79 7.6Z" stroke="currentColor" stroke-width="0.9"/>
              </svg>
            </button>
          </div>
          <!-- Stats view -->
          <template v-if="!showProjectSettings">
            <div class="tiles">
              <div class="tile tile-accent">
                <span class="tile-value">${{ taskStore.totalStats.cost_usd.toFixed(2) }}</span>
                <span class="tile-label">Cost</span>
              </div>
              <div class="tile">
                <span class="tile-value">{{ taskStore.totalStats.tasks_completed }}</span>
                <span class="tile-label">Done</span>
              </div>
              <div class="tile">
                <span class="tile-value">{{ taskStore.totalStats.tasks_failed }}</span>
                <span class="tile-label">Failed</span>
              </div>
              <div class="tile">
                <span class="tile-value">{{ taskStore.totalStats.num_turns }}</span>
                <span class="tile-label">Turns</span>
              </div>
              <div class="tile">
                <span class="tile-value">{{ formatTokens(taskStore.totalStats.output_tokens) }}</span>
                <span class="tile-label">Out tokens</span>
              </div>
              <div class="tile">
                <span class="tile-value">{{ formatTokens(taskStore.totalStats.input_tokens) }}</span>
                <span class="tile-label">In tokens</span>
              </div>
            </div>
          </template>

          <!-- Project settings view -->
          <template v-else>
            <div class="psettings">
              <div class="psettings-row">
                <span class="psettings-label">Effort</span>
                <div class="psettings-toggle">
                  <span v-for="level in ['low', 'medium', 'high', 'max']" :key="level"
                    class="psettings-opt"
                    :class="{ 'psettings-opt-active': projectEffort === level }"
                    @click="projectEffort = level"
                  >{{ level }}</span>
                </div>
              </div>
              <div class="psettings-row">
                <span class="psettings-label">Max turns</span>
                <input
                  type="number"
                  class="psettings-input"
                  v-model.number="projectMaxTurns"
                  min="1"
                  max="100"
                  placeholder="unlimited"
                />
              </div>
              <div class="psettings-row">
                <span class="psettings-label">Context</span>
                <button class="psettings-btn" @click="taskStore.resetSession(projectId)">Reset session</button>
              </div>
            </div>
          </template>
        </div>

        <!-- Current task box -->
        <div class="current-task" :class="{ 'current-task-max': outputMaximized, 'current-task-running': runningTask }">
          <div class="current-header">
            <div class="current-status">
              <template v-if="runningTask">
                <span class="running-dot"></span>
                <span class="running-label">Running</span>
              </template>
              <template v-else>
                <span class="idle-label">Idle</span>
              </template>
            </div>
            <span v-if="runningTask" class="current-title">{{ runningTask.title }}</span>
            <span v-else class="current-idle">No task running</span>
            <button v-if="runningTask" class="cancel-btn" @click="cancelTask" title="Cancel task">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              Cancel
            </button>
            <button class="expand-btn" @click="outputMaximized = !outputMaximized" :title="outputMaximized ? 'Collapse' : 'Expand'">
              <svg v-if="!outputMaximized" width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M7.5 1H11V4.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M4.5 11H1V7.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <svg v-else width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M4.5 1V4.5H1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M7.5 11V7.5H11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
          <div ref="outputEl" class="current-output">
            <template v-if="runningTask">
              <div v-for="(line, i) in liveOutput" :key="i" class="output-line">{{ line }}</div>
              <span class="output-cursor"></span>
            </template>
            <div v-else class="output-empty">Output will appear here when a task is running.</div>
          </div>
        </div>
      </div>

      <!-- Two draggable columns -->
      <div class="columns">
        <KanbanColumn
          title="Backlog"
          status="backlog"
          :tasks="backlogTasks"
          :allow-drag="true"
          @task-moved="handleTaskMoved"
          @open-detail="openDetail"
        >
          <template #actions>
            <button class="col-btn" @click="showAddDialog = true" title="Add task">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M6 1.5V10.5M1.5 6H10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
              </svg>
            </button>
          </template>
        </KanbanColumn>
        <KanbanColumn
          title="Queued"
          status="queued"
          :tasks="queuedTasks"
          :allow-drag="true"
          @task-moved="handleTaskMoved"
          @open-detail="openDetail"
        >
          <template #actions>
            <button
              class="col-btn"
              :class="queueRunning ? 'col-btn-stop' : 'col-btn-start'"
              @click="toggleQueue"
              :title="queueRunning ? 'Stop queue' : 'Start queue'"
            >
              <svg v-if="!queueRunning" width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M3 2L10 6L3 10V2Z" fill="currentColor"/>
              </svg>
              <svg v-else width="12" height="12" viewBox="0 0 12 12" fill="none">
                <rect x="2.5" y="2.5" width="7" height="7" rx="1" fill="currentColor"/>
              </svg>
            </button>
          </template>
        </KanbanColumn>
      </div>

      <!-- Collapsible history -->
      <div class="history">
        <button class="history-toggle" @click="historyOpen = !historyOpen">
          <svg
            width="10" height="10" viewBox="0 0 10 10" fill="none"
            class="history-chevron"
            :class="{ 'history-chevron-open': historyOpen }"
          >
            <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Completed
          <span class="history-badge">{{ historyCount }}</span>
        </button>
        <div v-if="historyOpen" class="history-list">
          <div v-if="historyCount === 0" class="history-empty">No completed tasks yet.</div>
          <div
            v-for="task in doneTasks"
            :key="task.id"
            class="history-item"
            @click="openDetail(task)"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="hi-icon hi-done">
              <path d="M2.5 6L5 8.5L9.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span class="hi-name">{{ task.title }}</span>
            <span class="hi-time">{{ formatDate(task.completed_at) }}</span>
          </div>
          <div
            v-for="task in failedTasks"
            :key="task.id"
            class="history-item"
            @click="openDetail(task)"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="hi-icon hi-failed">
              <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            </svg>
            <span class="hi-name">{{ task.title }}</span>
            <button class="hi-retry" @click.stop="retryTask(task.id)">Retry</button>
            <span class="hi-time">{{ formatDate(task.completed_at) }}</span>
          </div>
        </div>
      </div>
    </template>

    <TaskDetail
      :task="selectedTask"
      @close="selectedTask = null"
      @retry="retryTask($event)"
    />

    <AddTaskDialog
      :visible="showAddDialog"
      :project-id="projectId"
      @close="showAddDialog = false"
    />
  </div>
</template>

<style scoped>
/* Top row */
.top-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

/* Stats panel */
.stats-panel {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.stats-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
}

.stats-cog {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}

.stats-cog:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

/* Project settings */
.psettings {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.psettings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.psettings-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.psettings-toggle {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.psettings-opt {
  padding: 2px 10px;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  border-right: 1px solid var(--border);
  transition: color 0.15s ease, background 0.15s ease;
}

.psettings-opt:last-child {
  border-right: none;
}

.psettings-opt:hover {
  color: var(--text-secondary);
}

.psettings-opt-active {
  color: var(--text-primary);
  background: var(--bg-elevated);
}

.psettings-input {
  width: 70px;
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: inherit;
  font-size: 0.75rem;
  outline: none;
}

.psettings-input:focus {
  border-color: var(--accent);
}

.psettings-btn {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}

.psettings-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.tiles {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 6px;
}

.tile {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 4px;
  background: var(--bg-elevated);
  border-radius: var(--radius-xs);
}

.tile-accent {
  background: var(--hover-overlay);
}

.tile-value {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
  line-height: 1;
}

.tile-label {
  font-size: 0.5625rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

/* Current task box */
.current-task {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  display: flex;
  flex-direction: column;
}

.current-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}

.current-status {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.running-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #eab308;
  animation: pulse-dot 1.4s ease-in-out infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.running-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #eab308;
}

.idle-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
}

.current-title {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.current-idle {
  font-size: 0.8125rem;
  color: var(--text-muted);
}

.current-task-running {
  animation: pulse-border 2s ease-in-out infinite;
}

@keyframes pulse-border {
  0%, 100% { border-color: rgba(234, 179, 8, 0.4); }
  50% { border-color: rgba(234, 179, 8, 0.1); }
}

.current-task-max {
  position: fixed;
  inset: 0;
  z-index: 90;
  border-radius: 0;
  border: none;
  max-width: none;
}

.current-task-max .current-output {
  height: auto;
  flex: 1;
}

.expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s ease, background 0.15s ease;
}

.expand-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.cancel-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
  padding: 3px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s ease, background 0.15s ease;
}

.cancel-btn:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.current-output {
  height: 120px;
  overflow-y: auto;
  padding: 8px 14px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.6875rem;
  line-height: 1.6;
  background: var(--bg-terminal);
  border-radius: 0 0 var(--radius-xs) var(--radius-xs);
}

.output-line {
  color: var(--text-muted);
  white-space: pre-wrap;
  word-break: break-all;
}

.output-cursor {
  display: inline-block;
  width: 5px;
  height: 10px;
  background: var(--text-muted);
  animation: blink 1s step-end infinite;
}

@keyframes blink {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 0; }
}

.output-empty {
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
}

/* Git banner */
.git-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  background: rgba(234, 179, 8, 0.08);
  border: 1px solid rgba(234, 179, 8, 0.2);
  border-radius: var(--radius-xs);
  margin-bottom: 12px;
  flex-shrink: 0;
}

.git-banner-icon {
  color: #eab308;
  flex-shrink: 0;
}

.git-banner-text {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  flex: 1;
}

.git-init-btn {
  border: 1px solid var(--border-hover);
  background: transparent;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.git-init-btn:hover {
  background: var(--hover-overlay);
  border-color: var(--border-hover);
}

.git-init-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Column header buttons */
.col-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}
.col-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}
.col-btn-start { color: #22c55e; }
.col-btn-start:hover { color: #4ade80; background: rgba(34,197,94,0.1); }
.col-btn-stop { color: #f87171; }
.col-btn-stop:hover { color: #f87171; background: rgba(239,68,68,0.1); }

/* Columns */
.columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

/* History */
.history {
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  margin-top: 12px;
  padding-top: 8px;
}
.history-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s ease;
}
.history-toggle:hover { color: var(--text-secondary); }
.history-chevron { transition: transform 0.15s ease; }
.history-chevron-open { transform: rotate(90deg); }
.history-badge {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
  background: var(--hover-overlay);
  padding: 0 6px;
  border-radius: 100px;
}
.history-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding-top: 6px;
  max-height: 180px;
  overflow-y: auto;
}
.history-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s ease;
}
.history-item:hover { background: var(--bg-card); }
.hi-icon { flex-shrink: 0; }
.hi-done { color: #22c55e; }
.hi-failed { color: #f87171; }
.hi-name {
  flex: 1;
  font-size: 0.8125rem;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.hi-retry {
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  cursor: pointer;
  padding: 1px 6px;
  border-radius: 3px;
  transition: color 0.15s ease, background 0.15s ease;
  flex-shrink: 0;
}
.hi-retry:hover { color: var(--text-primary); background: var(--hover-overlay); }
.hi-time {
  font-size: 0.6875rem;
  color: var(--text-muted);
  flex-shrink: 0;
  white-space: nowrap;
}
.history-empty {
  font-size: 0.8125rem;
  color: var(--text-muted);
  padding: 4px 8px;
}
</style>
