<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTaskStore } from '../stores/taskStore'
import { useProjectStore } from '../stores/projectStore'
import KanbanColumn from '../components/KanbanColumn.vue'
import TaskCard from '../components/TaskCard.vue'
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
const gitInfo = ref<{ branch: string; changes: number; commits: { hash: string; message: string }[] } | null>(null)
// Progress bar state
const taskStartTime = ref<number | null>(null)
const outputCount = ref(0)
const progress = ref(0)
let progressTimer: ReturnType<typeof setInterval> | null = null

function startProgressTracking() {
  taskStartTime.value = Date.now()
  outputCount.value = 0
  progress.value = 0
  if (progressTimer) clearInterval(progressTimer)
  progressTimer = setInterval(updateProgress, 500)
}

function stopProgressTracking(completed: boolean) {
  if (completed) progress.value = 100
  if (progressTimer) {
    clearInterval(progressTimer)
    progressTimer = null
  }
}

function updateProgress() {
  if (!taskStartTime.value) return
  const elapsed = (Date.now() - taskStartTime.value) / 1000
  // Asymptotic curve: approaches 90% over time, never reaches it
  // Time component: logarithmic growth (fast initially, then slows)
  const timePart = 90 * (1 - Math.exp(-elapsed / 120))
  // Output activity bonus: each output line adds a tiny nudge (up to ~8%)
  const outputPart = Math.min(8, outputCount.value * 0.15)
  progress.value = Math.min(95, timePart + outputPart)
}

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
  loadGitInfo()
  queueRunning.value = false
  outputMaximized.value = false
  stopProgressTracking(false)
}

async function loadGitInfo() {
  if (!activeProject.value) return
  try {
    gitInfo.value = await invoke('get_git_info', { path: activeProject.value.path })
  } catch {
    gitInfo.value = null
  }
}

onMounted(loadProject)

const gitInterval = setInterval(() => {
  if (activeProject.value) {
    loadGitInfo()
  }
}, 30000)

onUnmounted(() => {
  clearInterval(gitInterval)
  if (progressTimer) clearInterval(progressTimer)
})

listen<{ project_id: string }>('queue-stopped', (event) => {
  if (event.payload.project_id === projectId.value) {
    queueRunning.value = false
    loadGitInfo()
  }
})

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
  taskStore.tasks.filter(t => t.status === 'done').sort((a, b) => new Date(b.completed_at ?? 0).getTime() - new Date(a.completed_at ?? 0).getTime())
)
const failedTasks = computed(() =>
  taskStore.tasks.filter(t => t.status === 'failed').sort((a, b) => new Date(b.completed_at ?? 0).getTime() - new Date(a.completed_at ?? 0).getTime())
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

// Progress bar watchers (must be after runningTask/liveOutput computed declarations)
watch(runningTask, (newVal, oldVal) => {
  if (newVal && !oldVal) {
    startProgressTracking()
  } else if (!newVal && oldVal) {
    stopProgressTracking(true)
  }
}, { immediate: true })

watch(() => liveOutput.value.length, (newLen, oldLen) => {
  if (newLen > oldLen) {
    outputCount.value = newLen
    updateProgress()
  }
})

async function handleTaskMoved(taskId: string, newStatus: Task['status'], newSortOrder: number) {
  await taskStore.moveTask(taskId, newStatus, newSortOrder)
}

async function handleDeleteTask(taskId: string) {
  await taskStore.deleteTask(taskId)
}

async function cancelTask() {
  if (!runningTask.value) return
  const taskId = runningTask.value.id
  stopProgressTracking(false)
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

async function moveToBacklog(taskId: string) {
  await handleTaskMoved(taskId, 'backlog', 0)
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
        <div class="stats-panel">
          <!-- Left: Git info -->
          <div class="stats-git">
            <div class="git-info" v-if="gitInfo">
              <div class="git-header">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="git-icon">
                  <circle cx="4" cy="4" r="1.5" stroke="currentColor" stroke-width="1.2"/>
                  <circle cx="10" cy="4" r="1.5" stroke="currentColor" stroke-width="1.2"/>
                  <circle cx="4" cy="10" r="1.5" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M4 5.5V8.5" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M10 5.5V7C10 7.55 9.55 8 9 8H4" stroke="currentColor" stroke-width="1.2"/>
                </svg>
                <span class="git-branch">{{ gitInfo.branch }}</span>
                <span v-if="gitInfo.changes > 0" class="git-changes">{{ gitInfo.changes }} change{{ gitInfo.changes === 1 ? '' : 's' }}</span>
                <button class="git-refresh-btn" @click="loadGitInfo" title="Refresh">
                  <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
                    <path d="M1.5 6A4.5 4.5 0 0 1 9.2 3M10.5 6A4.5 4.5 0 0 1 2.8 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                    <path d="M9.2 1V3H7.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M2.8 11V9H4.8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </div>
              <div class="git-commits">
                <div v-if="gitInfo.commits.length === 0" class="git-empty">No commits yet.</div>
                <div v-for="c in gitInfo.commits" :key="c.hash" class="git-commit">
                  <span class="git-hash">{{ c.hash }}</span>
                  <span class="git-msg">{{ c.message }}</span>
                </div>
              </div>
            </div>
            <div v-else class="git-info git-info-empty">
              <span class="git-empty">Git info unavailable</span>
            </div>
          </div>
        </div>

        <!-- Current task box -->
        <div class="current-task" :class="{ 'current-task-max': outputMaximized, 'current-task-running': runningTask }">
          <!-- Status bar -->
          <div class="current-bar">
            <div class="current-status">
              <template v-if="runningTask">
                <span class="running-dot"></span>
                <span class="running-label">Running</span>
              </template>
              <template v-else>
                <span class="idle-label">Idle</span>
              </template>
            </div>
            <div class="current-card-actions">
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
          </div>
          <!-- Progress bar -->
          <div v-if="runningTask" class="progress-track">
            <div class="progress-fill" :style="{ width: progress + '%' }"></div>
          </div>
          <!-- Card slot -->
          <div class="current-card-slot">
            <div v-if="runningTask" class="current-card-slot-inner">
              <TaskCard
                :task="runningTask"
                @open-detail="openDetail(runningTask!)"
              />
            </div>
            <div v-else class="current-card-slot-empty">
              <span class="current-idle">No task running</span>
            </div>
          </div>
          <!-- Output area -->
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
          :deletable="true"
          @task-moved="handleTaskMoved"
          @open-detail="openDetail"
          @delete-task="handleDeleteTask"
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
            <button class="hi-retry" @click.stop="moveToBacklog(task.id)">To Backlog</button>
            <span class="hi-time">{{ formatDate(task.completed_at) }}</span>
          </div>
        </div>
      </div>
    </template>

    <TaskDetail
      :task="selectedTask"
      @close="selectedTask = null"
      @retry="retryTask($event)"
      @move-to-backlog="moveToBacklog($event)"
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

/* Current task box */
.current-task {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  display: flex;
  flex-direction: column;
}

.current-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
}

.current-card-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.current-status {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

/* Progress bar */
.progress-track {
  height: 2px;
  background: rgba(234, 179, 8, 0.1);
  overflow: hidden;
  flex-shrink: 0;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, rgba(234, 179, 8, 0.6), #eab308);
  transition: width 0.6s ease-out;
  border-radius: 0 1px 1px 0;
}

.current-card-slot {
  padding: 10px;
  border-bottom: 1px solid var(--border);
}

.current-card-slot-inner :deep(.task-card) {
  margin: 0;
  box-shadow: var(--shadow-card);
}

.current-card-slot-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 4px;
  border: 1px dashed var(--border);
  border-radius: 4px;
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
  color: var(--text-secondary);
}

.current-idle {
  font-size: 0.8125rem;
  color: var(--text-secondary);
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
  color: var(--text-secondary);
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
  color: var(--text-secondary);
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
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-all;
}

.output-cursor {
  display: inline-block;
  width: 5px;
  height: 10px;
  background: var(--text-secondary);
  animation: blink 1s step-end infinite;
}

@keyframes blink {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 0; }
}

.output-empty {
  color: var(--text-secondary);
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
  color: var(--text-primary);
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
  color: var(--text-secondary);
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
  color: var(--text-secondary);
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.15s ease;
}
.history-toggle:hover { color: var(--text-primary); }
.history-chevron { transition: transform 0.15s ease; }
.history-chevron-open { transform: rotate(90deg); }
.history-badge {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-secondary);
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
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.hi-retry {
  border: none;
  background: transparent;
  color: var(--text-secondary);
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
  color: var(--text-secondary);
  flex-shrink: 0;
  white-space: nowrap;
}
.history-empty {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  padding: 4px 8px;
}

/* Git info panel */
.stats-panel {
  min-width: 0;
  display: flex;
  gap: 14px;
}

.stats-git {
  flex: 1;
  min-width: 0;
  width: 100%;
}

.git-info {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  display: flex;
  flex-direction: column;
  height: 100%;
}

.git-info-empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.git-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}

.git-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.git-branch {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.git-changes {
  font-size: 0.6875rem;
  font-weight: 500;
  color: #eab308;
  background: rgba(234, 179, 8, 0.12);
  padding: 1px 8px;
  border-radius: 100px;
  white-space: nowrap;
  flex-shrink: 0;
}

.git-refresh-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  flex-shrink: 0;
  margin-left: auto;
  transition: color 0.15s ease, background 0.15s ease;
}

.git-refresh-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.git-commits {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
}

.git-commit {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 2px 14px;
  font-size: 0.6875rem;
  line-height: 1.6;
}

.git-hash {
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.git-msg {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.git-empty {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  padding: 4px 14px;
}
</style>
