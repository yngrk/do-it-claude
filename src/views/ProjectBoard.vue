<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTaskStore } from '../stores/taskStore'
import { useProjectStore } from '../stores/projectStore'
import ProjectSettingsModal from '../components/ProjectSettingsModal.vue'
import KanbanColumn from '../components/KanbanColumn.vue'
import TaskCard from '../components/TaskCard.vue'
import TaskDetail from '../components/TaskDetail.vue'
import AddTaskDialog from '../components/AddTaskDialog.vue'
import type { Task } from '../types'

const route = useRoute()
const router = useRouter()
const taskStore = useTaskStore()
const projectStore = useProjectStore()
const projectId = computed(() => route.params.id as string)
const routeTaskId = computed(() => typeof route.query.task === 'string' ? route.query.task : null)
const routeFocus = computed<'details' | 'chat'>(() => route.query.focus === 'chat' ? 'chat' : 'details')
const showAddDialog = ref(false)
const showSettings = ref(false)
const showTemplateModal = ref(false)
const availableTemplates = ref<string[]>([])
const loadedTemplateName = ref<string | null>(null)
const selectedTask = ref<Task | null>(null)
const selectedTaskFocus = ref<'details' | 'chat'>('details')
const queueRunning = ref(false)
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

async function loadProject() {
  queueRunning.value = false
  outputMaximized.value = false
  stopProgressTracking(false)
  checkGitStatus()
  loadGitInfo()

  await projectStore.loadProjects()
  await taskStore.loadTasks(projectId.value)
  availableTemplates.value = await invoke<string[]>('list_templates')
  syncTaskFromRoute()
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

async function loadTemplate(name: string) {
  await invoke('load_template', { projectId: projectId.value, templateName: name })
  loadedTemplateName.value = name
  showTemplateModal.value = false
}

async function restoreBackup() {
  await invoke('restore_project_backup', { projectId: projectId.value })
  loadedTemplateName.value = null
  showTemplateModal.value = false
}

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

function openDetail(task: Task, focus: 'details' | 'chat' = 'details') {
  selectedTask.value = task
  selectedTaskFocus.value = focus
}

function syncTaskFromRoute() {
  if (!routeTaskId.value) return
  const task = taskStore.tasks.find(item => item.id === routeTaskId.value)
  if (task) {
    openDetail(task, routeFocus.value)
  }
}

async function closeDetail() {
  selectedTask.value = null
  selectedTaskFocus.value = 'details'

  if (!routeTaskId.value && !route.query.focus) return

  const nextQuery = { ...route.query }
  delete nextQuery.task
  delete nextQuery.focus
  await router.replace({ query: nextQuery })
}

async function retryTask(taskId: string) {
  await handleTaskMoved(taskId, 'queued', 0)
}

async function moveToBacklog(taskId: string) {
  await handleTaskMoved(taskId, 'backlog', 0)
}

watch(
  () => [routeTaskId.value, routeFocus.value, taskStore.tasks.length],
  () => {
    syncTaskFromRoute()
  },
  { immediate: true },
)

watch(
  () => taskStore.tasks,
  (tasks) => {
    if (!selectedTask.value) return
    const updated = tasks.find(task => task.id === selectedTask.value?.id) ?? null
    selectedTask.value = updated
  },
  { deep: true },
)

// Split resizer state
const splitRatio = ref(0.4)
const isDragging = ref(false)
const splitContainerEl = ref<HTMLElement | null>(null)

// Combined history for Done/Failed column
const historyTasks = computed(() =>
  taskStore.tasks
    .filter(t => t.status === 'done' || t.status === 'failed')
    .sort((a, b) => new Date(b.completed_at ?? 0).getTime() - new Date(a.completed_at ?? 0).getTime())
)

function formatTime(dateStr: string | null) {
  if (!dateStr) return ''
  const d = new Date(dateStr)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function onDividerMouseDown(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
  const startY = e.clientY
  const startRatio = splitRatio.value

  function onMouseMove(ev: MouseEvent) {
    if (!splitContainerEl.value) return
    const rect = splitContainerEl.value.getBoundingClientRect()
    const deltaRatio = (ev.clientY - startY) / rect.height
    splitRatio.value = Math.min(0.7, Math.max(0.2, startRatio + deltaRatio))
  }

  function onMouseUp() {
    isDragging.value = false
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

</script>

<template>
  <div class="project-board">
    <div v-if="taskStore.loading" class="loading">Loading tasks...</div>

    <template v-else>
      <!-- Project header with git badge, template badge, buttons -->
      <div class="project-header">
        <div class="project-header-left">
          <span class="project-header-name">{{ activeProject?.name }}</span>
          <div v-if="gitInfo" class="git-badge">
            <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
              <circle cx="4" cy="4" r="1.5" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="10" cy="4" r="1.5" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="4" cy="10" r="1.5" stroke="currentColor" stroke-width="1.2"/>
              <path d="M4 5.5V8.5" stroke="currentColor" stroke-width="1.2"/>
              <path d="M10 5.5V7C10 7.55 9.55 8 9 8H4" stroke="currentColor" stroke-width="1.2"/>
            </svg>
            <span class="git-badge-branch">{{ gitInfo.branch }}</span>
            <span v-if="gitInfo.changes > 0" class="git-badge-changes">{{ gitInfo.changes }}</span>
            <button class="git-refresh-btn" @click="loadGitInfo" title="Refresh git info">
              <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                <path d="M1.5 6A4.5 4.5 0 0 1 9.2 3M10.5 6A4.5 4.5 0 0 1 2.8 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                <path d="M9.2 1V3H7.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M2.8 11V9H4.8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
        </div>
        <div class="project-header-right">
          <span v-if="loadedTemplateName" class="template-badge">
            <svg width="10" height="10" viewBox="0 0 14 14" fill="none">
              <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
            </svg>
            {{ loadedTemplateName }}
          </span>
          <button class="template-btn" @click="showTemplateModal = true">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M3 1h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V3a2 2 0 0 1 2-2Z" stroke="currentColor" stroke-width="1.2"/>
              <path d="M5 4h4M5 7h4M5 10h2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
            {{ loadedTemplateName ? 'Change' : 'Load Template' }}
          </button>
          <button class="settings-btn" @click="showSettings = true">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M6.5 1L6.1 2.7a5.5 5.5 0 0 0-1.3.8L3.2 2.9l-1.5 2.6 1.5 1.3a5.5 5.5 0 0 0 0 1.4L1.7 9.5l1.5 2.6 1.6-.6c.4.3.8.6 1.3.8L6.5 15h3l.4-1.7c.5-.2.9-.5 1.3-.8l1.6.6 1.5-2.6-1.5-1.3a5.5 5.5 0 0 0 0-1.4l1.5-1.3-1.5-2.6-1.6.6a5.5 5.5 0 0 0-1.3-.8L9.5 1h-3ZM8 5.5a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            Settings
          </button>
        </div>
      </div>

      <!-- Git warning banner -->
      <div v-if="isGit === false" class="git-banner">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="git-banner-icon">
          <path d="M7 1L1 13H13L7 1Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
          <path d="M7 5.5V8.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <circle cx="7" cy="10.5" r="0.5" fill="currentColor"/>
        </svg>
        <span class="git-banner-text">Not a git repository. Cancelling tasks won't be able to revert changes.</span>
        <button class="git-init-btn" @click="initGit" :disabled="initingGit">
          {{ initingGit ? 'Initializing...' : 'Initialize git' }}
        </button>
      </div>

      <!-- Split container: execution monitor + divider + task board -->
      <div ref="splitContainerEl" class="split-container">
        <!-- Execution Monitor (top) -->
        <div class="execution-monitor" :class="{ 'monitor-running': runningTask }" :style="{ height: (splitRatio * 100) + '%' }">
          <div class="monitor-content">
            <!-- Left: task info -->
            <div class="monitor-left">
              <div class="monitor-status-line">
                <template v-if="runningTask">
                  <span class="running-dot"></span>
                  <span class="running-label">Running</span>
                </template>
                <template v-else>
                  <span class="idle-label">Idle</span>
                </template>
                <button v-if="runningTask" class="cancel-btn" @click="cancelTask" title="Cancel task">
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                  </svg>
                  Cancel
                </button>
              </div>
              <div v-if="runningTask" class="progress-track">
                <div class="progress-fill" :style="{ width: progress + '%' }"></div>
              </div>
              <div v-if="runningTask" class="monitor-task-card">
                <TaskCard :task="runningTask" @open-detail="openDetail(runningTask!)" />
              </div>
              <div v-else class="monitor-idle">
                <span>Queue tasks and hit play</span>
              </div>
            </div>
            <!-- Right: output -->
            <div class="monitor-right">
              <div class="monitor-output-header">
                <span class="monitor-output-label">Output</span>
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
        </div>

        <!-- Resizable divider -->
        <div class="split-divider" :class="{ dragging: isDragging }" @mousedown="onDividerMouseDown"></div>

        <!-- Task Board (bottom) — 3 columns -->
        <div class="task-board">
          <div class="board-columns">
            <KanbanColumn title="Backlog" status="backlog" :tasks="backlogTasks" :allow-drag="true" :deletable="true" @task-moved="handleTaskMoved" @open-detail="openDetail" @delete-task="handleDeleteTask">
              <template #actions>
                <button class="col-btn" @click="showAddDialog = true" title="Add task">
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <path d="M6 1.5V10.5M1.5 6H10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                  </svg>
                </button>
              </template>
            </KanbanColumn>
            <KanbanColumn title="Queued" status="queued" :tasks="queuedTasks" :allow-drag="true" @task-moved="handleTaskMoved" @open-detail="openDetail">
              <template #actions>
                <button class="col-btn" :class="queueRunning ? 'col-btn-stop' : 'col-btn-start'" @click="toggleQueue" :title="queueRunning ? 'Stop queue' : 'Start queue'">
                  <svg v-if="!queueRunning" width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M3 2L10 6L3 10V2Z" fill="currentColor"/></svg>
                  <svg v-else width="12" height="12" viewBox="0 0 12 12" fill="none"><rect x="2.5" y="2.5" width="7" height="7" rx="1" fill="currentColor"/></svg>
                </button>
              </template>
            </KanbanColumn>
            <!-- Done / Failed column -->
            <div class="kanban-column column-history">
              <div class="column-header">
                <div class="column-header-left">
                  <span class="column-title">DONE / FAILED</span>
                  <span class="column-count">{{ historyCount }}</span>
                </div>
              </div>
              <div class="history-list">
                <div v-if="historyTasks.length === 0" class="empty-hint">No completed tasks yet</div>
                <div v-for="task in historyTasks" :key="task.id" class="history-row" @click="openDetail(task)">
                  <svg v-if="task.status === 'done'" width="12" height="12" viewBox="0 0 12 12" fill="none" class="history-icon history-icon-done">
                    <path d="M2.5 6L5 8.5L9.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <svg v-else width="12" height="12" viewBox="0 0 12 12" fill="none" class="history-icon history-icon-failed">
                    <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                  </svg>
                  <span class="history-row-name">{{ task.title }}</span>
                  <div v-if="task.status === 'failed'" class="history-row-actions">
                    <button class="history-action-btn" @click.stop="retryTask(task.id)">Retry</button>
                    <button class="history-action-btn" @click.stop="moveToBacklog(task.id)">Backlog</button>
                  </div>
                  <span class="history-row-time">{{ formatTime(task.completed_at) }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Maximized output overlay -->
      <div v-if="outputMaximized" class="output-maximized-overlay">
        <div class="output-max-header">
          <span class="monitor-output-label">Output</span>
          <div style="display: flex; align-items: center; gap: 8px;">
            <template v-if="runningTask">
              <span class="running-dot"></span>
              <span class="running-label">Running</span>
            </template>
            <button class="expand-btn" @click="outputMaximized = false" title="Collapse">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M4.5 1V4.5H1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M7.5 11V7.5H11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
        </div>
        <div ref="outputEl" class="current-output output-max-content">
          <template v-if="runningTask">
            <div v-for="(line, i) in liveOutput" :key="i" class="output-line">{{ line }}</div>
            <span class="output-cursor"></span>
          </template>
          <div v-else class="output-empty">Output will appear here when a task is running.</div>
        </div>
      </div>
    </template>

    <!-- Modals -->
    <TaskDetail :task="selectedTask" :initial-focus="selectedTaskFocus" @close="closeDetail" @retry="retryTask($event)" @move-to-backlog="moveToBacklog($event)" />
    <AddTaskDialog :visible="showAddDialog" :project-id="projectId" @close="showAddDialog = false" />
    <ProjectSettingsModal :visible="showSettings" :project-id="projectId" @close="showSettings = false" />

    <!-- Load Template Modal -->
    <div v-if="showTemplateModal" class="modal-overlay" @click.self="showTemplateModal = false">
      <div class="modal template-modal slide-up">
        <div class="modal-header">
          <h2>Load Template</h2>
          <button class="btn-icon" @click="showTemplateModal = false" title="Close">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-body template-list">
          <p class="template-desc">Load a template to set up CLAUDE.md and agent definitions in this project. Your existing config will be backed up.</p>
          <div v-if="availableTemplates.length === 0" class="template-empty">No templates found</div>
          <button v-for="name in availableTemplates" :key="name" class="template-item" @click="loadTemplate(name)">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="template-item-icon">
              <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            <span class="template-item-name">{{ name }}</span>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="template-item-arrow">
              <path d="M4.5 2.5L8 6L4.5 9.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="restoreBackup">Restore Original</button>
          <button class="btn btn-secondary" @click="showTemplateModal = false">Cancel</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Project header */
.project-header { display: flex; align-items: center; justify-content: space-between; padding: 2px 0 8px; flex-shrink: 0; }
.project-header-left { display: flex; align-items: center; gap: 10px; min-width: 0; }
.project-header-name { font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: 0.875rem; font-weight: 500; letter-spacing: -0.01em; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.project-header-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

/* Template badge */
.template-badge { display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; background: rgba(168, 85, 247, 0.1); border: 1px solid rgba(168, 85, 247, 0.3); border-radius: 100px; font-size: 0.6875rem; font-weight: 600; color: #c084fc; white-space: nowrap; flex-shrink: 0; }
.template-badge svg { color: #c084fc; }

/* Header buttons */
.template-btn { display: inline-flex; align-items: center; gap: 6px; padding: 4px 12px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-card); color: var(--text-secondary); font-family: inherit; font-size: 0.75rem; font-weight: 500; cursor: pointer; flex-shrink: 0; transition: all 0.15s ease; }
.template-btn:hover { color: #60a5fa; border-color: rgba(59, 130, 246, 0.4); background: rgba(59, 130, 246, 0.08); box-shadow: 0 0 8px rgba(59, 130, 246, 0.1); }
.settings-btn { display: inline-flex; align-items: center; gap: 6px; padding: 4px 12px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-card); color: var(--text-secondary); font-family: inherit; font-size: 0.75rem; font-weight: 500; cursor: pointer; flex-shrink: 0; transition: all 0.15s ease; }
.settings-btn:hover { color: #c084fc; border-color: rgba(168, 85, 247, 0.4); background: rgba(168, 85, 247, 0.1); box-shadow: 0 0 8px rgba(168, 85, 247, 0.1); }

/* Git badge */
.git-badge { display: inline-flex; align-items: center; gap: 6px; padding: 2px 10px; background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.2); border-radius: 100px; font-size: 0.6875rem; flex-shrink: 0; }
.git-badge svg { color: #60a5fa; flex-shrink: 0; }
.git-badge-branch { font-weight: 600; color: var(--text-primary); }
.git-badge-changes { font-weight: 600; color: #60a5fa; background: rgba(59, 130, 246, 0.2); padding: 0 6px; border-radius: 100px; min-width: 16px; text-align: center; }
.git-refresh-btn { display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border: none; border-radius: 3px; background: transparent; color: var(--text-secondary); cursor: pointer; flex-shrink: 0; transition: color 0.15s ease; }
.git-refresh-btn:hover { color: var(--text-primary); }

/* Git banner */
.git-banner { display: flex; align-items: center; gap: 10px; padding: 8px 14px; background: rgba(234, 179, 8, 0.08); border: 1px solid rgba(234, 179, 8, 0.2); border-radius: var(--radius-xs); margin-bottom: 12px; flex-shrink: 0; }
.git-banner-icon { color: #eab308; flex-shrink: 0; }
.git-banner-text { font-size: 0.8125rem; color: var(--text-primary); flex: 1; }
.git-init-btn { border: 1px solid var(--border-hover); background: transparent; color: var(--text-primary); font-family: inherit; font-size: 0.6875rem; font-weight: 500; padding: 3px 10px; border-radius: 4px; cursor: pointer; white-space: nowrap; flex-shrink: 0; transition: background 0.15s ease, border-color 0.15s ease; }
.git-init-btn:hover { background: var(--hover-overlay); border-color: var(--border-hover); }
.git-init-btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* Split container */
.split-container { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: hidden; }

/* Execution monitor */
.execution-monitor { flex: none; display: flex; flex-direction: column; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius-xs); overflow: hidden; min-height: 120px; }
.monitor-running { border-color: rgba(168, 85, 247, 0.5); box-shadow: 0 0 24px rgba(168, 85, 247, 0.12), inset 0 0 30px rgba(168, 85, 247, 0.03); animation: pulse-border 2s ease-in-out infinite; }
@keyframes pulse-border {
  0%, 100% { border-color: rgba(168, 85, 247, 0.6); box-shadow: 0 0 24px rgba(168, 85, 247, 0.15), inset 0 0 30px rgba(168, 85, 247, 0.03); }
  50% { border-color: rgba(99, 102, 241, 0.4); box-shadow: 0 0 16px rgba(99, 102, 241, 0.08); }
}
.monitor-content { flex: 1; display: flex; min-height: 0; }
.monitor-left { width: 280px; flex-shrink: 0; display: flex; flex-direction: column; border-right: 1px solid var(--border); padding: 12px 14px; gap: 10px; }
.monitor-status-line { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.running-dot { width: 8px; height: 8px; border-radius: 50%; background: #a855f7; box-shadow: 0 0 8px rgba(168, 85, 247, 0.6); animation: pulse-dot 1.4s ease-in-out infinite; }
@keyframes pulse-dot { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
.running-label { font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #c084fc; }
.idle-label { font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-secondary); }
.cancel-btn { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; padding: 3px 10px; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); font-family: inherit; font-size: 0.6875rem; font-weight: 500; cursor: pointer; flex-shrink: 0; transition: color 0.15s ease, background 0.15s ease; }
.cancel-btn:hover { color: #fb7185; background: rgba(251, 113, 133, 0.15); box-shadow: 0 0 8px rgba(251, 113, 133, 0.1); }

/* Progress bar */
.progress-track { height: 4px; background: rgba(168, 85, 247, 0.15); overflow: hidden; flex-shrink: 0; }
.progress-fill { height: 100%; background: linear-gradient(90deg, #a855f7, #6366f1, #3b82f6); box-shadow: 0 0 12px rgba(168, 85, 247, 0.5); transition: width 0.6s ease-out; border-radius: 0 2px 2px 0; }
.monitor-task-card { flex: 1; min-height: 0; overflow-y: auto; }
.monitor-task-card :deep(.task-card) { margin: 0; box-shadow: none; }
.monitor-idle { flex: 1; display: flex; align-items: center; justify-content: center; color: var(--text-muted); font-size: 0.8125rem; }

/* Monitor right (output) */
.monitor-right { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.monitor-output-header { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
.monitor-output-label { font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-secondary); }
.expand-btn { display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); cursor: pointer; flex-shrink: 0; transition: color 0.15s ease, background 0.15s ease; }
.expand-btn:hover { color: #c084fc; background: rgba(168, 85, 247, 0.1); }
.current-output { flex: 1; min-height: 0; overflow-y: auto; padding: 8px 14px; font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: 0.6875rem; line-height: 1.6; background: var(--bg-terminal); }
.output-line { color: var(--text-secondary); white-space: pre-wrap; word-break: break-all; }
.output-cursor { display: inline-block; width: 6px; height: 12px; background: #a855f7; box-shadow: 0 0 6px rgba(168, 85, 247, 0.5); animation: blink 1s step-end infinite; }
@keyframes blink { 0%, 100% { opacity: 0.7; } 50% { opacity: 0; } }
.output-empty { color: var(--text-secondary); font-family: inherit; font-size: 0.6875rem; }

/* Split divider */
.split-divider { height: 8px; flex-shrink: 0; cursor: row-resize; display: flex; align-items: center; justify-content: center; position: relative; user-select: none; }
.split-divider::after { content: ''; width: 40px; height: 3px; border-radius: 2px; background: var(--border); transition: background 0.15s ease, box-shadow 0.15s ease; }
.split-divider:hover::after, .split-divider.dragging::after { background: #a855f7; box-shadow: 0 0 8px rgba(168, 85, 247, 0.3); }

/* Task board */
.task-board { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.board-columns { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 14px; flex: 1; min-height: 0; }

/* Column header buttons */
.col-btn { display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); cursor: pointer; transition: color 0.15s ease, background 0.15s ease; }
.col-btn:hover { color: var(--text-primary); background: var(--hover-overlay); }
.col-btn-start { color: #34d399; }
.col-btn-start:hover { color: #6ee7b7; background: rgba(52, 211, 153, 0.15); box-shadow: 0 0 8px rgba(52, 211, 153, 0.15); }
.col-btn-stop { color: #fb7185; }
.col-btn-stop:hover { color: #fb7185; background: rgba(251, 113, 133, 0.15); box-shadow: 0 0 8px rgba(251, 113, 133, 0.1); }

/* Done/Failed column */
.column-history { display: flex; flex-direction: column; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-xs); border-top: 3px solid #10b981; overflow: hidden; }
.column-history .column-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; min-height: 44px; border-bottom: 1px solid var(--border); flex-shrink: 0; background: linear-gradient(180deg, rgba(16, 185, 129, 0.08) 0%, transparent 100%); }
.column-header-left { display: flex; align-items: center; gap: 8px; }
.column-title { font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: #34d399; }
.column-count { font-size: 0.6875rem; font-weight: 600; color: var(--text-secondary); background: var(--hover-overlay); padding: 0 6px; border-radius: 100px; min-width: 18px; text-align: center; }
.column-history .history-list { flex: 1; overflow-y: auto; padding: 6px; display: flex; flex-direction: column; gap: 2px; }
.empty-hint { padding: 24px 12px; color: var(--text-muted); font-size: 0.8rem; text-align: center; user-select: none; }
.history-row { display: flex; align-items: center; gap: 8px; padding: 5px 8px; border-radius: 4px; cursor: pointer; transition: background 0.15s ease; }
.history-row:hover { background: rgba(168, 85, 247, 0.06); }
.history-icon { flex-shrink: 0; }
.history-icon-done { color: #34d399; filter: drop-shadow(0 0 3px rgba(52, 211, 153, 0.4)); }
.history-icon-failed { color: #fb7185; filter: drop-shadow(0 0 3px rgba(251, 113, 133, 0.4)); }
.history-row-name { flex: 1; font-size: 0.8125rem; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.history-row-actions { display: flex; gap: 2px; flex-shrink: 0; }
.history-action-btn { border: none; background: transparent; color: var(--text-secondary); font-family: inherit; font-size: 0.6875rem; font-weight: 500; cursor: pointer; padding: 1px 6px; border-radius: 3px; transition: color 0.15s ease, background 0.15s ease; flex-shrink: 0; }
.history-action-btn:hover { color: #c084fc; background: rgba(168, 85, 247, 0.12); }
.history-row-time { font-size: 0.6875rem; color: var(--text-muted); flex-shrink: 0; white-space: nowrap; }

/* Maximized output overlay */
.output-maximized-overlay { position: fixed; inset: 0; z-index: 90; background: var(--bg-base); display: flex; flex-direction: column; }
.output-max-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; border-bottom: 1px solid var(--border); background: var(--bg-card); flex-shrink: 0; }
.output-max-content { flex: 1; border-radius: 0; }

/* Template modal */
.template-modal { width: 100%; max-width: 420px; }
.template-list { display: flex; flex-direction: column; gap: 6px; }
.template-desc { font-size: 0.8125rem; color: var(--text-muted); line-height: 1.5; margin-bottom: 6px; }
.template-empty { font-size: 0.8125rem; color: var(--text-muted); text-align: center; padding: 20px; }
.template-item { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border: 1px solid var(--border); border-radius: var(--radius-xs); background: var(--bg-card); color: var(--text-primary); font-family: inherit; font-size: 0.875rem; font-weight: 500; cursor: pointer; transition: all 0.15s ease; }
.template-item:hover { border-color: rgba(168, 85, 247, 0.4); background: rgba(168, 85, 247, 0.06); box-shadow: 0 0 12px rgba(168, 85, 247, 0.08); }
.template-item-icon { color: var(--text-muted); flex-shrink: 0; }
.template-item:hover .template-item-icon { color: #c084fc; }
.template-item-name { flex: 1; }
.template-item-arrow { color: var(--text-muted); opacity: 0; transition: opacity 0.15s ease, transform 0.15s ease; }
.template-item:hover .template-item-arrow { opacity: 1; transform: translateX(2px); }
</style>
