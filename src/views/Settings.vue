<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useTheme } from '../composables/useTheme'

const router = useRouter()
const { theme, setTheme } = useTheme()

interface ClaudeStatus {
  found: boolean
  path: string
  version: string
}

const claude = ref<ClaudeStatus | null>(null)
const checking = ref(false)

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

onMounted(() => {
  checkClaude()
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
  </div>
</template>

<style scoped>
.settings-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.settings-sections {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 600px;
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

.settings-label {
  font-size: 0.875rem;
  color: var(--text-secondary);
  min-width: 100px;
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

.check-ok { background: #22c55e; }
.check-fail { background: #f87171; }
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

.check-text-ok { color: #22c55e; }
.check-text-fail { color: #f87171; }

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
</style>
