<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useChatStore } from '../stores/chatStore'
import { useTaskStore } from '../stores/taskStore'
import type { ProjectMessage } from '../types'

const props = defineProps<{ projectId: string }>()

const chatStore = useChatStore()
const taskStore = useTaskStore()
const inputEl = ref<HTMLTextAreaElement | null>(null)
const imageInputEl = ref<HTMLInputElement | null>(null)
const docInputEl = ref<HTMLInputElement | null>(null)
const messagesEl = ref<HTMLElement | null>(null)
const input = ref('')
const attachments = ref<{ data: string; media_type: string; name: string; preview: string | null }[]>([])
const showAttachMenu = ref(false)
const showTaskForm = ref(false)
const taskTitle = ref('')
const taskDescription = ref('')

const messages = computed(() => chatStore.messages[props.projectId] || [])
const draft = computed(() => chatStore.liveDraft[props.projectId] || '')
const isSending = computed(() => chatStore.sending[props.projectId] || false)
const error = computed(() => chatStore.errors[props.projectId] || null)

function onClickOutside(e: MouseEvent) {
  if (showAttachMenu.value) {
    const wrapper = (e.target as HTMLElement)?.closest('.attach-wrapper')
    if (!wrapper) showAttachMenu.value = false
  }
}

onMounted(() => {
  chatStore.loadMessages(props.projectId)
  document.addEventListener('click', onClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', onClickOutside)
})

watch(() => props.projectId, () => {
  chatStore.loadMessages(props.projectId)
})

// Auto-scroll on new messages or draft changes
watch([() => messages.value.length, draft], async () => {
  await nextTick()
  if (messagesEl.value) {
    messagesEl.value.scrollTop = messagesEl.value.scrollHeight
  }
})

async function send() {
  const text = input.value.trim()
  if ((!text && attachments.value.length === 0) || isSending.value) return
  const files = attachments.value.map(({ data, media_type, name }) => ({ data, media_type, name }))
  input.value = ''
  attachments.value = []
  await chatStore.sendMessage(props.projectId, text, files)
}

function handlePaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items
  if (!items) return
  for (const item of items) {
    if (item.type.startsWith('image/')) {
      e.preventDefault()
      const file = item.getAsFile()
      if (file) addFile(file)
      return
    }
  }
}

function handleImageUpload(e: Event) {
  const el = e.target as HTMLInputElement
  if (!el.files) return
  for (const file of el.files) addFile(file)
  el.value = ''
  showAttachMenu.value = false
}

function handleDocUpload(e: Event) {
  const el = e.target as HTMLInputElement
  if (!el.files) return
  for (const file of el.files) addFile(file)
  el.value = ''
  showAttachMenu.value = false
}

function addFile(file: File) {
  const reader = new FileReader()
  reader.onload = () => {
    const result = reader.result as string
    const base64 = result.split(',')[1]
    const isImage = file.type.startsWith('image/')
    attachments.value = [...attachments.value, {
      data: base64,
      media_type: file.type || 'application/octet-stream',
      name: file.name,
      preview: isImage ? result : null,
    }]
  }
  reader.readAsDataURL(file)
}

function removeAttachment(index: number) {
  attachments.value = attachments.value.filter((_, i) => i !== index)
}

function toggleAttachMenu() {
  showAttachMenu.value = !showAttachMenu.value
}

function pickImage() {
  imageInputEl.value?.click()
}

function pickDocument() {
  docInputEl.value?.click()
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}

function prefillTask(msg: ProjectMessage) {
  taskDescription.value = msg.content
  taskTitle.value = ''
  showTaskForm.value = true
}

async function createTask() {
  if (!taskTitle.value.trim()) return
  await taskStore.createTask(props.projectId, taskTitle.value.trim(), taskDescription.value.trim())
  showTaskForm.value = false
  taskTitle.value = ''
  taskDescription.value = ''
}

async function clearChat() {
  await chatStore.clearChat(props.projectId)
}

interface ParsedTask {
  title: string
  description: string
  tag: string
}

function parseMessage(content: string): { text: string; tasks: ParsedTask[] } {
  const tasks: ParsedTask[] = []
  let text = content
  const regex = /<tasks>([\s\S]*?)<\/tasks>/g
  let match
  while ((match = regex.exec(content)) !== null) {
    try {
      const parsed = JSON.parse(match[1].trim())
      if (Array.isArray(parsed)) {
        tasks.push(...parsed)
      }
    } catch { /* ignore parse errors */ }
    text = text.replace(match[0], '')
  }
  return { text: text.trim(), tasks }
}
</script>

<template>
  <div class="idea-chat">
    <div class="chat-header">
      <div class="chat-header-left">
        <span class="chat-title">Planning</span>
        <span class="chat-count">{{ messages.length }}</span>
      </div>
      <div class="chat-header-actions">
        <button v-if="messages.length > 0" class="col-btn" @click="clearChat" title="Clear chat">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2 3H10M4.5 3V2H7.5V3M3 3V10C3 10.55 3.45 11 4 11H8C8.55 11 9 10.55 9 10V3" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
    </div>

    <div ref="messagesEl" class="chat-messages">
      <div v-if="messages.length === 0 && !draft" class="chat-empty">
        <div class="chat-empty-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v10Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
          </svg>
        </div>
        <span>Plan features, break down work, create tasks</span>
      </div>

      <template v-for="msg in messages" :key="msg.id">
        <div :class="['chat-msg', `chat-msg-${msg.role}`]">
          <div class="msg-bubble">
            <div class="msg-content">{{ msg.role === 'assistant' ? parseMessage(msg.content).text : msg.content }}</div>
            <!-- Render created tasks as visual cards -->
            <div v-if="msg.role === 'assistant' && parseMessage(msg.content).tasks.length > 0" class="msg-tasks">
              <div class="msg-tasks-label">
                <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                  <path d="M2.5 6L5 8.5L9.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                {{ parseMessage(msg.content).tasks.length }} task{{ parseMessage(msg.content).tasks.length > 1 ? 's' : '' }} added to queue
              </div>
              <div v-for="(t, i) in parseMessage(msg.content).tasks" :key="i" class="msg-task-card">
                <span v-if="t.tag" class="msg-task-tag">{{ t.tag }}</span>
                <span class="msg-task-title">{{ t.title }}</span>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- Streaming draft -->
      <div v-if="draft" class="chat-msg chat-msg-assistant">
        <div class="msg-bubble">
          <div class="msg-content">{{ draft }}</div>
          <span class="msg-cursor"></span>
        </div>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="chat-error">
      <span>{{ error }}</span>
    </div>

    <!-- Attachments preview -->
    <div v-if="attachments.length > 0" class="chat-attachments">
      <div v-for="(att, i) in attachments" :key="i" class="chat-attachment" :class="{ 'chat-attachment-doc': !att.preview }">
        <img v-if="att.preview" :src="att.preview" class="attachment-thumb" />
        <div v-else class="attachment-doc">
          <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
            <path d="M3 1H9L12 4V13H3V1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            <path d="M9 1V4H12" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
          </svg>
          <span class="attachment-doc-name">{{ att.name }}</span>
        </div>
        <button class="attachment-remove" @click="removeAttachment(i)">
          <svg width="8" height="8" viewBox="0 0 12 12" fill="none">
            <path d="M2 2L10 10M10 2L2 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Inline task creation form -->
    <div v-if="showTaskForm" class="task-form">
      <div class="task-form-header">
        <span class="task-form-title">Create Task</span>
        <button class="col-btn" @click="showTaskForm = false">
          <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
            <path d="M2 2L10 10M10 2L2 10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <input
        v-model="taskTitle"
        type="text"
        class="task-form-input"
        placeholder="Task title..."
        @keydown.enter="createTask"
      />
      <textarea
        v-model="taskDescription"
        class="task-form-textarea"
        placeholder="Task prompt..."
        rows="3"
      />
      <button class="task-form-submit" :disabled="!taskTitle.trim()" @click="createTask">
        Add to Queue
      </button>
    </div>

    <!-- Input area -->
    <div class="chat-input-area">
      <div class="attach-wrapper">
        <button class="chat-attach-btn" @click="toggleAttachMenu" :disabled="isSending" title="Attach file">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M7 1V13M1 7H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
        <div v-if="showAttachMenu" class="attach-menu">
          <button class="attach-menu-item" @click="pickImage">
            <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
              <rect x="1" y="1" width="12" height="12" rx="2" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="4.5" cy="4.5" r="1.5" stroke="currentColor" stroke-width="1"/>
              <path d="M1 10L4.5 7L7 9.5L9.5 7L13 10" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            Image
          </button>
          <button class="attach-menu-item" @click="pickDocument">
            <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
              <path d="M3 1H9L12 4V13H3V1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
              <path d="M9 1V4H12" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            Document
          </button>
        </div>
      </div>
      <input ref="imageInputEl" type="file" accept="image/*" multiple hidden @change="handleImageUpload" />
      <input ref="docInputEl" type="file" accept=".pdf,.txt,.md,.csv,.json,.xml,.yaml,.yml,.toml,.log,.html,.css,.js,.ts,.py,.rs,.go,.java,.c,.cpp,.h,.rb,.sh" multiple hidden @change="handleDocUpload" />
      <textarea
        ref="inputEl"
        v-model="input"
        class="chat-input"
        placeholder="What do you want to build?"
        rows="1"
        :disabled="isSending"
        @keydown="handleKeydown"
        @paste="handlePaste"
      />
      <button class="chat-send-btn" :disabled="(!input.trim() && attachments.length === 0) || isSending" @click="send">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 7H12M12 7L8 3M12 7L8 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.idea-chat {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 44px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.chat-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.chat-title {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #c084fc;
}

.chat-count {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-secondary);
  background: var(--hover-overlay);
  padding: 0 6px;
  border-radius: 100px;
  min-width: 18px;
  text-align: center;
}

.chat-header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

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
.col-btn:hover { color: var(--text-primary); background: var(--hover-overlay); }

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chat-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 0.75rem;
  user-select: none;
}

.chat-empty-icon { opacity: 0.3; }

.chat-msg {
  display: flex;
  flex-direction: column;
  max-width: 90%;
}

.chat-msg-user {
  align-self: flex-end;
}

.chat-msg-user .msg-bubble {
  background: rgba(167, 139, 250, 0.15);
  border: 1px solid rgba(167, 139, 250, 0.2);
  border-radius: 12px 12px 4px 12px;
}

.chat-msg-assistant {
  align-self: flex-start;
}

.chat-msg-assistant .msg-bubble {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 12px 12px 12px 4px;
}

.msg-bubble {
  padding: 8px 12px;
}

.msg-content {
  font-size: 0.8125rem;
  line-height: 1.55;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.msg-cursor {
  display: inline-block;
  width: 5px;
  height: 11px;
  background: #a78bfa;
  animation: blink 1s step-end infinite;
  vertical-align: text-bottom;
  margin-left: 2px;
}
@keyframes blink { 0%, 100% { opacity: 0.7; } 50% { opacity: 0; } }

.msg-actions {
  display: flex;
  gap: 4px;
  padding: 2px 4px;
}

.msg-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 2px 8px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
}
.msg-action-btn:hover {
  color: #c084fc;
  background: rgba(168, 85, 247, 0.1);
}

/* Tasks created from chat */
.msg-tasks {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.msg-tasks-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.6875rem;
  font-weight: 600;
  color: #4ade80;
  margin-bottom: 2px;
}

.msg-task-card {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.15);
  border-radius: 4px;
}

.msg-task-tag {
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-muted);
  flex-shrink: 0;
}

.msg-task-title {
  font-size: 0.75rem;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-error {
  padding: 6px 12px;
  background: rgba(248, 113, 113, 0.1);
  border-top: 1px solid rgba(248, 113, 113, 0.2);
  color: #f87171;
  font-size: 0.75rem;
  flex-shrink: 0;
}

/* Inline task form */
.task-form {
  padding: 10px 12px;
  border-top: 1px solid var(--border);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex-shrink: 0;
}

.task-form-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.task-form-title {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-secondary);
}

.task-form-input {
  background: var(--bg-card);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 6px 10px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: inherit;
  outline: none;
  width: 100%;
}
.task-form-input:focus { border-color: var(--accent); }
.task-form-input::placeholder { color: var(--text-muted); }

.task-form-textarea {
  background: var(--bg-card);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 6px 10px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: inherit;
  outline: none;
  resize: vertical;
  min-height: 48px;
  width: 100%;
  line-height: 1.5;
}
.task-form-textarea:focus { border-color: var(--accent); }
.task-form-textarea::placeholder { color: var(--text-muted); }

.task-form-submit {
  align-self: flex-end;
  padding: 4px 14px;
  border: none;
  border-radius: var(--radius-xs);
  background: var(--accent);
  color: #fff;
  font-family: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s ease;
}
.task-form-submit:disabled { opacity: 0.4; cursor: not-allowed; }
.task-form-submit:not(:disabled):hover { opacity: 0.85; }

/* Chat input */
.chat-input-area {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  padding: 8px 10px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.chat-input {
  flex: 1;
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: inherit;
  line-height: 1.5;
  outline: none;
  resize: none;
  max-height: 120px;
}
.chat-input:focus { border-color: var(--accent); }
.chat-input::placeholder { color: var(--text-muted); }
.chat-input:disabled { opacity: 0.5; }

.chat-send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  align-self: stretch;
  border: none;
  border-radius: var(--radius-xs);
  background: var(--accent);
  color: #fff;
  cursor: pointer;
  flex-shrink: 0;
  transition: opacity 0.15s ease;
}
.chat-send-btn:disabled { opacity: 0.3; cursor: not-allowed; }
.chat-send-btn:not(:disabled):hover { opacity: 0.85; }

.chat-attachments {
  display: flex;
  gap: 6px;
  padding: 6px 10px 0;
  flex-wrap: wrap;
  flex-shrink: 0;
}

.chat-attachment {
  position: relative;
  width: 48px;
  height: 48px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border);
}

.chat-attachment-doc {
  width: auto;
  max-width: 160px;
  height: auto;
  padding: 0;
}

.attachment-thumb {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.attachment-doc {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 24px 6px 8px;
  color: var(--text-secondary);
}

.attachment-doc-name {
  font-size: 0.6875rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-remove {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 16px;
  height: 16px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.7);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
}
.attachment-remove:hover {
  background: rgba(220, 38, 38, 0.9);
}

.attach-wrapper {
  position: relative;
  align-self: stretch;
  display: flex;
}

.chat-attach-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  align-self: stretch;
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s ease, border-color 0.15s ease;
}
.chat-attach-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}
.chat-attach-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.attach-menu {
  position: absolute;
  bottom: 100%;
  left: 0;
  margin-bottom: 4px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  min-width: 120px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10;
}

.attach-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  font-family: inherit;
  font-size: 0.75rem;
  cursor: pointer;
  white-space: nowrap;
  transition: color 0.15s ease, background 0.15s ease;
}
.attach-menu-item:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}
</style>
