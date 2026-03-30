<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useTaskStore } from '../stores/taskStore'
import { DEFAULT_TASK_TAGS } from '../types'
import type { Task, TokenEstimate } from '../types'
import { formatDuration } from '../utils/dateFormat'
import TaskDefinitionPane from './task-detail/TaskDefinitionPane.vue'
import TaskChatPane from './task-detail/TaskChatPane.vue'
import TaskExecutionPane from './task-detail/TaskExecutionPane.vue'
import TaskMetadataPane from './task-detail/TaskMetadataPane.vue'

type ActivityTab = 'chat' | 'execution' | 'details'

const props = defineProps<{ task: Task | null; initialFocus?: 'details' | 'chat' }>()
const emit = defineEmits<{ close: []; retry: [taskId: string]; 'move-to-backlog': [taskId: string] }>()

const taskStore = useTaskStore()

const editTitle = ref('')
const editDescription = ref('')
const editTag = ref<string | null>(null)
const editModel = ref<string | null>(null)
const tokenEstimate = ref<TokenEstimate | null>(null)
const saving = ref(false)
const includeExecutionInChat = ref(false)
const chatInput = ref('')
const logsExpanded = ref(false)
const provider = ref<'claude' | 'codex'>('claude')
const activeTab = ref<ActivityTab>('details')

async function loadProvider() {
  try {
    const saved = await invoke<string | null>('get_setting', { key: 'cli_provider' })
    if (saved === 'codex') provider.value = 'codex'
  } catch {}
}

const providerLabel = computed(() => provider.value === 'codex' ? 'Codex' : 'Claude')
const isEditable = computed(() => props.task?.status === 'backlog' || props.task?.status === 'queued')

const modelOptions = computed(() => {
  if (provider.value === 'codex') {
    return [
      { value: '', label: 'Auto' },
      { value: 'gpt-5.4', label: 'GPT-5.4' },
      { value: 'gpt-5.4-mini', label: 'GPT-5.4 Mini' },
    ]
  }

  return [
    { value: '', label: 'Auto' },
    { value: 'claude-sonnet-4-6', label: 'Sonnet' },
    { value: 'claude-opus-4-6', label: 'Opus' },
  ]
})

const statusLabel = computed(() => props.task?.status.replace('_', ' ') ?? 'task')
const activeTaskProviderLabel = computed(() => {
  if (props.task?.provider === 'codex') return 'Codex'
  if (props.task?.provider === 'claude') return 'Claude'
  return providerLabel.value
})

const taskChip = computed(() => {
  if (!props.task?.tag) return null
  return DEFAULT_TASK_TAGS.find(tag => tag.value === props.task?.tag) ?? null
})

const duration = computed(() => formatDuration(props.task?.started_at ?? null, props.task?.completed_at ?? null))

const logs = computed(() => {
  if (!props.task) return []
  return taskStore.taskLogs[props.task.id] || []
})

const liveOutput = computed(() => {
  if (!props.task) return []
  return taskStore.liveLogs[props.task.id] || []
})

const messages = computed(() => {
  if (!props.task) return []
  return taskStore.taskMessages[props.task.id] || []
})

const chatDraft = computed(() => {
  if (!props.task) return ''
  return taskStore.liveChatDrafts[props.task.id] || ''
})

const isChatSending = computed(() => {
  if (!props.task) return false
  return taskStore.chatSending[props.task.id] || false
})

const chatError = computed(() => {
  if (!props.task) return null
  return taskStore.chatErrors[props.task.id] || null
})

const chatDisabledReason = computed(() => {
  if (props.task?.status === 'in_progress') {
    return 'Chat is unavailable while this task is actively running.'
  }
  return null
})

const transcriptEntries = computed(() => {
  if (!props.task) return []

  const chatEntries = messages.value.map(message => ({
    id: message.id,
    role: message.role,
    content: message.content,
    kind: 'chat' as const,
    created_at: message.created_at,
  }))

  if (!includeExecutionInChat.value) return chatEntries

  const logEntries = logs.value.map(log => ({
    id: `log-${log.id}`,
    role: 'execution' as const,
    content: log.content,
    kind: 'execution' as const,
    created_at: log.created_at,
    tone: log.log_type,
  }))

  return [...chatEntries, ...logEntries].sort((a, b) =>
    new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
  )
})

const latestSignal = computed(() => {
  if (liveOutput.value.length > 0) return liveOutput.value[liveOutput.value.length - 1]
  if (logs.value.length > 0) return logs.value[logs.value.length - 1].content
  return null
})

const executionSummary = computed(() => {
  if (!props.task) return ''
  if (props.task.status === 'in_progress') {
    return `${activeTaskProviderLabel.value} is currently working through this task. Live output will keep updating here while the run is active.`
  }
  if (props.task.status === 'failed') {
    return `${activeTaskProviderLabel.value} finished this run with a failure state${props.task.exit_code !== null ? ` (exit ${props.task.exit_code})` : ''}. Review the latest signal and logs before retrying or asking for a follow-up fix.`
  }
  if (props.task.status === 'done') {
    return `${activeTaskProviderLabel.value} completed this task successfully. Use chat to inspect what changed or execution logs to audit the full run.`
  }
  return `This task is ready for editing. Refine the prompt on the left, then queue it when the definition is solid.`
})

const isDirty = computed(() => {
  if (!props.task) return false
  return editTitle.value !== props.task.title
    || editDescription.value !== props.task.description
    || (editTag.value || null) !== (props.task.tag || null)
})

const headerSubtitle = computed(() => {
  if (!props.task) return ''
  const parts = [statusLabel.value, activeTaskProviderLabel.value]
  const model = modelOptions.value.find(option => option.value === props.task?.model)?.label ?? props.task.model
  if (model) parts.push(model)
  if (duration.value) parts.push(duration.value)
  return parts.join(' · ')
})

function defaultTab(task: Task | null, focus?: 'details' | 'chat'): ActivityTab {
  if (!task) return 'details'
  if (focus === 'chat') return 'chat'
  if (task.status === 'in_progress' || task.status === 'failed') return 'execution'
  if (task.status === 'done') return 'chat'
  return 'details'
}

async function save() {
  if (!props.task || !editTitle.value.trim()) return
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

async function saveModel() {
  if (!props.task) return
  await taskStore.updateTaskModel(props.task.id, editModel.value)
}

async function sendChatMessage() {
  if (!props.task || !chatInput.value.trim() || isChatSending.value || chatDisabledReason.value) return
  const content = chatInput.value
  chatInput.value = ''
  try {
    await taskStore.sendTaskMessage(props.task.id, content)
  } catch {
    chatInput.value = content
  }
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (!props.task) return
  if (event.key === 'Escape') {
    emit('close')
    return
  }

  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's' && isEditable.value && isDirty.value) {
    event.preventDefault()
    save()
  }
}

onMounted(() => {
  loadProvider()
  window.addEventListener('keydown', handleWindowKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleWindowKeydown)
})

watch(() => props.task, (task) => {
  logsExpanded.value = false
  chatInput.value = ''
  if (!task) return

  taskStore.loadTaskLogs(task.id)
  taskStore.loadTaskMessages(task.id)
  editTitle.value = task.title
  editDescription.value = task.description
  editTag.value = task.tag || null
  editModel.value = task.model ?? null
  taskStore.estimateTaskTokens(task.id).then(est => {
    tokenEstimate.value = est
  }).catch(() => {
    tokenEstimate.value = null
  })
}, { immediate: true })

watch(
  () => [props.task?.id, props.initialFocus],
  (current, previous) => {
    const [taskId, focus] = current as [string | undefined, 'details' | 'chat' | undefined]
    const [previousTaskId, previousFocus] = (previous ?? []) as [string | undefined, 'details' | 'chat' | undefined]
    if (!taskId || !props.task) return
    if (taskId !== previousTaskId || focus !== previousFocus) {
      activeTab.value = defaultTab(props.task, focus)
    }
  },
  { immediate: true },
)

watch(
  () => [
    activeTab.value,
    transcriptEntries.value.length,
    chatDraft.value.length,
    liveOutput.value.length,
    includeExecutionInChat.value,
  ],
  async ([tab]) => {
    if (tab !== 'chat') return
    await nextTick()
    const transcript = document.querySelector<HTMLElement>('.workspace-panel .chat-box')
    if (transcript) transcript.scrollTop = transcript.scrollHeight
  },
)
</script>

<template>
  <div v-if="task" class="modal-overlay task-workspace-overlay" @click.self="emit('close')">
    <div class="modal workspace-modal slide-up">
      <header class="workspace-header">
        <div class="workspace-header-main">
          <div class="workspace-title-row">
            <span :class="`workspace-status workspace-status-${task.status}`"></span>
            <h2 class="workspace-title">{{ task.title }}</h2>
            <span v-if="taskChip" class="workspace-tag" :style="{ background: `${taskChip.color}1e`, borderColor: `${taskChip.color}44`, color: taskChip.color }">
              <span class="workspace-tag-dot" :style="{ background: taskChip.color }"></span>
              {{ taskChip.label }}
            </span>
          </div>
          <p class="workspace-subtitle">{{ headerSubtitle }}</p>
        </div>

        <div class="workspace-actions">
          <button
            v-if="task.status === 'done' || task.status === 'failed'"
            class="btn btn-secondary btn-sm"
            @click="emit('move-to-backlog', task.id)"
          >
            Move to Queue
          </button>
          <button
            v-if="task.status === 'failed'"
            class="btn btn-secondary btn-sm"
            @click="emit('retry', task.id)"
          >
            Retry
          </button>
          <button
            v-if="isEditable && isDirty"
            class="btn btn-primary btn-sm"
            :disabled="!editTitle.trim() || saving"
            @click="save"
          >
            {{ saving ? 'Saving...' : 'Save' }}
          </button>
          <button class="btn btn-secondary btn-sm" @click="emit('close')">Close</button>
        </div>
      </header>

      <div class="workspace-body">
        <aside class="workspace-sidebar">
          <TaskDefinitionPane
            :task="task"
            :is-editable="isEditable"
            :provider-label="providerLabel"
            :edit-title="editTitle"
            :edit-description="editDescription"
            :edit-tag="editTag"
            :edit-model="editModel"
            :model-options="modelOptions"
            :token-estimate="tokenEstimate"
            @update:title="editTitle = $event"
            @update:description="editDescription = $event"
            @update:tag="editTag = $event"
            @update:model="editModel = $event"
            @save-model="saveModel"
          />
        </aside>

        <section class="workspace-panel">
          <div class="workspace-tabs">
            <button
              class="workspace-tab"
              :class="{ 'workspace-tab-active': activeTab === 'chat' }"
              @click="activeTab = 'chat'"
            >
              Chat
            </button>
            <button
              class="workspace-tab"
              :class="{ 'workspace-tab-active': activeTab === 'execution' }"
              @click="activeTab = 'execution'"
            >
              Execution
            </button>
            <button
              class="workspace-tab"
              :class="{ 'workspace-tab-active': activeTab === 'details' }"
              @click="activeTab = 'details'"
            >
              Details
            </button>
          </div>

          <div
            class="workspace-panel-body"
            :class="{ 'workspace-panel-body-scroll': activeTab !== 'chat' }"
          >
            <TaskChatPane
              v-if="activeTab === 'chat'"
              :provider-label="providerLabel"
              :transcript-entries="transcriptEntries"
              :chat-draft="chatDraft"
              :is-chat-sending="isChatSending"
              :chat-error="chatError"
              :chat-disabled-reason="chatDisabledReason"
              :include-execution-in-chat="includeExecutionInChat"
              :live-output="task.status === 'in_progress' ? liveOutput : []"
              :chat-input="chatInput"
              @update:includeExecutionInChat="includeExecutionInChat = $event"
              @update:chatInput="chatInput = $event"
              @send="sendChatMessage"
            />

            <TaskExecutionPane
              v-else-if="activeTab === 'execution'"
              :task="task"
              :provider-label="providerLabel"
              :duration="duration"
              :live-output="liveOutput"
              :logs="logs"
              :logs-expanded="logsExpanded"
              :summary-text="executionSummary"
              :latest-signal="latestSignal"
              @toggle-logs="logsExpanded = !logsExpanded"
            />

            <TaskMetadataPane
              v-else
              :task="task"
              :provider-label="providerLabel"
              :duration="duration"
            />
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-workspace-overlay {
  align-items: stretch;
  padding: 24px;
}

.workspace-modal {
  width: min(1180px, 100%);
  height: min(88vh, calc(100vh - 48px));
  max-height: min(88vh, calc(100vh - 48px));
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.workspace-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  padding: 20px 22px 18px;
  border-bottom: 1px solid var(--border);
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--accent) 6%, transparent), transparent 42%),
    var(--bg-card);
}

.workspace-header-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.workspace-title-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.workspace-title {
  margin: 0;
  font-size: 1.2rem;
  line-height: 1.2;
  color: var(--text-primary);
}

.workspace-subtitle {
  margin: 0;
  font-size: 0.84rem;
  color: var(--text-muted);
}

.workspace-status {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.workspace-status-backlog { background: var(--badge-backlog-color); }
.workspace-status-queued { background: var(--badge-queued-color); }
.workspace-status-in_progress { background: var(--badge-in_progress-color); animation: pulse-glow 1.6s ease infinite; }
.workspace-status-done { background: var(--badge-done-color); }
.workspace-status-failed { background: var(--badge-failed-color); }

.workspace-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid;
  font-size: 0.74rem;
  font-weight: 600;
}

.workspace-tag-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.workspace-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.workspace-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(320px, 0.9fr) minmax(0, 1.4fr);
  overflow: hidden;
}

.workspace-sidebar {
  min-width: 0;
  min-height: 0;
  padding: 22px;
  overflow-y: auto;
  border-right: 1px solid var(--border);
  background: linear-gradient(180deg, color-mix(in srgb, var(--bg-elevated) 72%, transparent), transparent 30%);
}

.workspace-panel {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-card);
}

.workspace-tabs {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 10px 14px 0;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.workspace-tab {
  padding: 8px 10px 9px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 0.76rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  cursor: pointer;
}

.workspace-tab-active {
  color: var(--text-primary);
  border-bottom-color: var(--accent);
}

.workspace-panel-body {
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  padding: 14px;
}

.workspace-panel-body-scroll {
  overflow-y: auto;
}

@media (max-width: 980px) {
  .task-workspace-overlay {
    padding: 12px;
  }

  .workspace-modal {
    width: 100%;
    height: min(100%, calc(100vh - 24px));
    max-height: min(100%, calc(100vh - 24px));
  }

  .workspace-body {
    grid-template-columns: 1fr;
  }

  .workspace-sidebar {
    border-right: none;
    border-bottom: 1px solid var(--border);
    max-height: 42vh;
  }

  .workspace-header {
    flex-direction: column;
  }

  .workspace-actions {
    justify-content: flex-start;
  }
}
</style>
