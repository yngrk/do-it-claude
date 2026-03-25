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
const notificationPermission = ref<'granted' | 'denied' | 'default'>('default')

const configurations = ref<string[]>([])
const loadingConfig = ref(false)

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

async function loadConfigurations() {
  loadingConfig.value = true
  try {
    configurations.value = await invoke<string[]>('list_templates')
  } finally {
    loadingConfig.value = false
  }
}

async function openConfigurationsFolder() {
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
  loadConfigurations()
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

    <div class="settings-grid">
      <!-- Left column: compact panels -->
      <div class="settings-col">
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
      </div>

      <!-- Right column: larger panels -->
      <div class="settings-col">
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
              <button class="refresh-btn" @click="testNotifications" :disabled="!settings.enabled">Test</button>
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
              <input type="checkbox" :checked="settings.notify_on_task_done" :disabled="!settings.enabled" @change="updateChecked('notify_on_task_done', checkboxChecked($event))" />
              <span>Task completion</span>
            </label>
            <label class="settings-checkbox">
              <input type="checkbox" :checked="settings.notify_on_task_failed" :disabled="!settings.enabled" @change="updateChecked('notify_on_task_failed', checkboxChecked($event))" />
              <span>Task failure</span>
            </label>
            <label class="settings-checkbox">
              <input type="checkbox" :checked="settings.notify_on_chat_reply" :disabled="!settings.enabled" @change="updateChecked('notify_on_chat_reply', checkboxChecked($event))" />
              <span>Claude chat replies</span>
            </label>
            <label class="settings-checkbox">
              <input type="checkbox" :checked="settings.suppress_when_focused" :disabled="!settings.enabled" @change="updateChecked('suppress_when_focused', checkboxChecked($event))" />
              <span>Suppress when focused</span>
            </label>
          </div>
        </section>

        <section class="glass-panel settings-section">
          <h2 class="settings-section-title">Configurations</h2>

          <div class="config-list">
            <div v-if="loadingConfig" class="config-empty">Loading...</div>
            <template v-else-if="configurations.length > 0">
              <div v-for="name in configurations" :key="name" class="config-item">
                <svg width="13" height="13" viewBox="0 0 14 14" fill="none" class="config-item-icon">
                  <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
                </svg>
                <span class="config-item-name">{{ name }}</span>
              </div>
            </template>
            <div v-else class="config-empty">No configurations found.</div>
          </div>

          <div class="settings-inline" style="margin-top: 12px;">
            <button class="open-folder-btn" @click="openConfigurationsFolder">
              <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
                <path d="M2 2h4l1.5 1.5H12a1 1 0 0 1 1 1V11a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
              </svg>
              Open Folder
            </button>
            <button class="recheck-btn" @click="loadConfigurations" :disabled="loadingConfig">Refresh</button>
          </div>
        </section>
      </div>
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

.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  align-items: start;
}

.settings-col {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

@media (max-width: 560px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
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

.config-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.config-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}

.config-item:last-child {
  border-bottom: none;
}

.config-item-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.config-item-name {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

.config-empty {
  font-size: 0.8125rem;
  color: var(--text-muted);
  padding: 4px 0;
}

.open-folder-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
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

.refresh-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
