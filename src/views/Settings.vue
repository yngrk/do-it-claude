<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted } from '@tauri-apps/plugin-notification'
import { useTheme } from '../composables/useTheme'
import { useNotificationSettings } from '../composables/useNotificationSettings'
import { requestNotificationAccess, sendTestNotification } from '../services/notifications'

const router = useRouter()
const { theme, setTheme } = useTheme()
const { settings, update } = useNotificationSettings()

interface ClaudeStatus {
  found: boolean
  path: string
  version: string
}

const claude = ref<ClaudeStatus | null>(null)
const checking = ref(false)
const templates = ref<string[]>([])
const notificationPermission = ref<'granted' | 'denied' | 'default'>('default')

async function checkClaude() {
  checking.value = true
  try {
    claude.value = await invoke<ClaudeStatus>('check_claude')
  } catch {
    claude.value = { found: false, path: '', version: '' }
  } finally {
    checking.value = false
  }
}

async function loadTemplates() {
  templates.value = await invoke<string[]>('list_templates')
}

async function openTemplatesFolder() {
  await invoke('open_templates_folder')
}

async function refreshNotificationPermission() {
  notificationPermission.value = await isPermissionGranted() ? 'granted' : Notification.permission
}

async function setNotificationsEnabled(value: boolean) {
  if (!value) {
    update({ enabled: false })
    return
  }

  const granted = await requestNotificationAccess()
  notificationPermission.value = granted ? 'granted' : 'denied'
  update({ enabled: granted })
}

async function testNotifications() {
  await sendTestNotification(router)
}

function checkboxChecked(event: Event) {
  return (event.target as HTMLInputElement).checked
}

function updateChecked<K extends keyof typeof settings.value>(key: K, checked: boolean) {
  update({ [key]: checked })
}

onMounted(() => {
  checkClaude()
  loadTemplates()
  refreshNotificationPermission()
})

function goBack() {
  router.push('/')
}
</script>

<template>
  <div class="settings-page fade-in">
    <div class="settings-header">
      <button class="tab-action" @click="goBack" title="Back">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
          <path d="M10 3L5 8L10 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <h1>Settings</h1>
    </div>

    <div class="settings-sections">
      <section class="glass-panel settings-section">
        <h2 class="settings-section-title">Claude CLI</h2>

        <div class="claude-check">
          <div class="check-status">
            <span v-if="checking" class="check-dot check-loading"></span>
            <span v-else-if="claude?.found" class="check-dot check-ok"></span>
            <span v-else class="check-dot check-fail"></span>

            <span v-if="checking" class="check-text">Checking...</span>
            <span v-else-if="claude?.found" class="check-text check-text-ok">Found</span>
            <span v-else class="check-text check-text-fail">Not found</span>

            <button class="recheck-btn" @click="checkClaude" :disabled="checking">Recheck</button>
          </div>

          <template v-if="claude?.found">
            <div class="check-detail">
              <span class="check-label">Path</span>
              <code class="check-value">{{ claude.path }}</code>
            </div>
            <div class="check-detail">
              <span class="check-label">Version</span>
              <code class="check-value">{{ claude.version }}</code>
            </div>
          </template>

          <p v-else-if="claude && !claude.found" class="check-help">
            Install Claude Code and ensure <code>claude</code> is in your PATH.
          </p>
        </div>
      </section>

      <section class="glass-panel settings-section">
        <h2 class="settings-section-title">Appearance</h2>
        <div class="settings-row">
          <span class="settings-label">Theme</span>
          <div class="theme-toggle">
            <span class="theme-option" :class="{ 'theme-active': theme === 'dark' }" @click="setTheme('dark')">Dark</span>
            <span class="theme-option" :class="{ 'theme-active': theme === 'light' }" @click="setTheme('light')">Light</span>
            <span class="theme-option theme-option-unhinged" :class="{ 'theme-active': theme === 'unhinged' }" @click="setTheme('unhinged')">Unhinged</span>
          </div>
        </div>
      </section>

      <section class="glass-panel settings-section">
        <h2 class="settings-section-title">Notifications</h2>

        <div class="settings-row settings-row-top">
          <span class="settings-label">Enable</span>
          <div class="settings-inline">
            <label class="settings-checkbox">
              <input
                type="checkbox"
                :checked="settings.enabled"
                @change="setNotificationsEnabled(checkboxChecked($event))"
              />
              <span>OS notifications</span>
            </label>
            <button class="refresh-btn" @click="testNotifications" :disabled="!settings.enabled">Test notification</button>
          </div>
        </div>

        <div class="settings-row">
          <span class="settings-label">Permission</span>
          <div class="settings-inline">
            <span class="text-muted">{{ notificationPermission }}</span>
            <button class="recheck-btn" @click="refreshNotificationPermission">Refresh</button>
          </div>
        </div>

        <div class="settings-option-list">
          <label class="settings-checkbox">
            <input
              type="checkbox"
              :checked="settings.notify_on_task_done"
              :disabled="!settings.enabled"
              @change="updateChecked('notify_on_task_done', checkboxChecked($event))"
            />
            <span>Notify on task completion</span>
          </label>
          <label class="settings-checkbox">
            <input
              type="checkbox"
              :checked="settings.notify_on_task_failed"
              :disabled="!settings.enabled"
              @change="updateChecked('notify_on_task_failed', checkboxChecked($event))"
            />
            <span>Notify on task failure</span>
          </label>
          <label class="settings-checkbox">
            <input
              type="checkbox"
              :checked="settings.notify_on_chat_reply"
              :disabled="!settings.enabled"
              @change="updateChecked('notify_on_chat_reply', checkboxChecked($event))"
            />
            <span>Notify on Claude chat replies</span>
          </label>
          <label class="settings-checkbox">
            <input
              type="checkbox"
              :checked="settings.suppress_when_focused"
              :disabled="!settings.enabled"
              @change="updateChecked('suppress_when_focused', checkboxChecked($event))"
            />
            <span>Suppress while app is focused</span>
          </label>
        </div>

        <p class="templates-desc settings-help">
          Notifications open the relevant project and task when clicked. Task starts, queue stop events, and chat streaming chunks do not notify.
        </p>
      </section>

      <section class="glass-panel settings-section">
        <h2 class="settings-section-title">About</h2>
        <div class="settings-row">
          <span class="settings-label">App</span>
          <span class="settings-app-name">Do It Claude</span>
        </div>
        <div class="settings-row">
          <span class="settings-label">Version</span>
          <span class="text-muted">0.2.0</span>
        </div>
      </section>

      <section class="glass-panel settings-section">
        <h2 class="settings-section-title">Templates</h2>

        <div class="templates-info">
          <p class="templates-desc">
            Templates are folders that contain <code>CLAUDE.md</code> and <code>.claude/agents/</code> files.
            When you load a template into a project, these files are copied into the project directory so Claude Code picks them up natively.
          </p>
          <div class="templates-how">
            <span class="templates-how-title">How to create a template:</span>
            <ol class="templates-steps">
              <li>Open the templates folder</li>
              <li>Create a new folder with the template name</li>
              <li>Add a <code>CLAUDE.md</code> with your system instructions</li>
              <li>Optionally add <code>.claude/agents/*.md</code> files for custom agent definitions</li>
            </ol>
          </div>
        </div>

        <div class="templates-list">
          <div v-for="name in templates" :key="name" class="template-list-item">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="template-list-icon">
              <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            <span class="template-list-name">{{ name }}</span>
          </div>
          <p v-if="templates.length === 0" class="templates-empty">No templates found. Open the folder to create one.</p>
        </div>

        <div class="templates-actions">
          <button class="open-folder-btn" @click="openTemplatesFolder">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
            </svg>
            Open Templates Folder
          </button>
          <button class="refresh-btn" @click="loadTemplates">Refresh</button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 100%;
  height: 100%;
  overflow-y: auto;
}

.settings-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.settings-sections {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}

.settings-sections .settings-section {
  flex: 1 1 300px;
  min-width: 280px;
}

.settings-section {
  padding: 20px 24px;
}

.settings-section-title {
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 16px;
}

.settings-row {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 12px;
}

.settings-row:last-child {
  margin-bottom: 0;
}

.settings-row-top {
  align-items: flex-start;
}

.settings-label {
  font-size: 0.875rem;
  color: var(--text-secondary);
  min-width: 100px;
}

.settings-inline {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.settings-option-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.settings-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.settings-checkbox input {
  accent-color: var(--accent);
}

.settings-help {
  margin-top: 12px;
}

.settings-app-name {
  font-weight: 600;
  font-size: 1rem;
  color: var(--text-primary);
}

.text-muted {
  color: var(--text-muted);
}

/* Theme toggle */
.theme-toggle {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
  cursor: pointer;
}

.theme-option {
  padding: 4px 14px;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-muted);
  transition: color 0.15s ease, background 0.15s ease;
}

.theme-option {
  border-right: 1px solid var(--border);
}

.theme-option:last-child {
  border-right: none;
}

.theme-active {
  color: var(--text-primary);
  background: var(--bg-elevated);
}

/* Claude check */
.claude-check {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.check-status {
  display: flex;
  align-items: center;
  gap: 8px;
}

.check-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.check-ok { background: var(--success); }
.check-fail { background: var(--error); }
.check-loading {
  background: #777;
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.check-text {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-secondary);
}

.check-text-ok { color: var(--success); }
.check-text-fail { color: var(--error); }

.recheck-btn {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  font-weight: 500;
  cursor: pointer;
  padding: 2px 8px;
  border-radius: 4px;
  transition: color 0.15s ease, background 0.15s ease;
}

.recheck-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.recheck-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.check-detail {
  display: flex;
  align-items: center;
  gap: 12px;
}

.check-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  min-width: 60px;
}

.check-value {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.check-help {
  font-size: 0.8125rem;
  color: var(--text-muted);
}

.check-help code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

/* Templates section */
.templates-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
}

.templates-desc {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  line-height: 1.6;
  margin: 0;
}

.templates-desc code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
  color: #c084fc;
  background: rgba(168, 85, 247, 0.1);
  padding: 1px 5px;
  border-radius: 3px;
}

.templates-how {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
}

.templates-how-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-primary);
  display: block;
  margin-bottom: 8px;
}

.templates-steps {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.templates-steps li {
  font-size: 0.75rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

.templates-steps code {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  color: #c084fc;
  background: rgba(168, 85, 247, 0.1);
  padding: 1px 4px;
  border-radius: 3px;
}

.templates-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 14px;
}

.template-list-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-surface);
}

.template-list-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.template-list-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
}

.templates-empty {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin: 0;
  padding: 8px 0;
}

.templates-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.open-folder-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: 1px solid rgba(168, 85, 247, 0.3);
  border-radius: 6px;
  background: rgba(168, 85, 247, 0.08);
  color: #c084fc;
  font-family: inherit;
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.open-folder-btn:hover {
  background: rgba(168, 85, 247, 0.15);
  border-color: rgba(168, 85, 247, 0.5);
  box-shadow: 0 0 12px rgba(168, 85, 247, 0.1);
}

.open-folder-btn svg {
  color: #c084fc;
}

.refresh-btn {
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-family: inherit;
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.refresh-btn:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}
</style>
