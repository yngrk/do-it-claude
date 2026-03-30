<script setup lang="ts">
import type { Task, TaskLog } from '../../types'

defineProps<{
  task: Task
  providerLabel: string
  duration: string | null
  liveOutput: string[]
  logs: TaskLog[]
  logsExpanded: boolean
  summaryText: string
  latestSignal: string | null
}>()

const emit = defineEmits<{
  'toggle-logs': []
}>()
</script>

<template>
  <section class="execution-pane">
    <div class="summary-card">
      <div class="summary-header">
        <span class="pane-kicker">Execution</span>
        <span class="status-pill" :class="`status-pill-${task.status}`">{{ task.status.replace('_', ' ') }}</span>
      </div>
      <p class="summary-copy">{{ summaryText }}</p>
      <div class="summary-metrics">
        <div class="metric-item">
          <span class="metric-label">Provider</span>
          <span class="metric-value">{{ task.provider === 'codex' ? 'Codex' : task.provider === 'claude' ? 'Claude' : providerLabel }}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Model</span>
          <span class="metric-value">{{ task.model || 'Auto' }}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Duration</span>
          <span class="metric-value">{{ duration || 'Not finished' }}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Exit</span>
          <span class="metric-value">{{ task.exit_code ?? 'Running' }}</span>
        </div>
      </div>
      <div v-if="latestSignal" class="signal-box">
        <span class="signal-label">Latest signal</span>
        <div class="signal-content">{{ latestSignal }}</div>
      </div>
    </div>

    <div v-if="task.status === 'in_progress' && liveOutput.length > 0" class="log-panel">
      <div class="panel-header">
        <span class="panel-title"><span class="live-dot"></span>Live Output</span>
      </div>
      <div class="log-box">
        <div v-for="(line, index) in liveOutput" :key="index" class="log-line">{{ line }}</div>
        <span class="cursor"></span>
      </div>
    </div>

    <div v-if="logs.length > 0" class="log-panel">
      <button class="panel-header panel-button" @click="emit('toggle-logs')">
        <span class="panel-title">Execution Logs</span>
        <span class="panel-meta">{{ logsExpanded ? 'Hide' : 'Show' }} · {{ logs.length }}</span>
      </button>
      <div v-if="logsExpanded" class="log-box">
        <div
          v-for="log in logs"
          :key="log.id"
          class="log-line"
          :class="`log-${log.log_type}`"
        >
          {{ log.content }}
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.execution-pane {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.summary-card,
.log-panel {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
}

.summary-card {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.summary-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.pane-kicker,
.panel-title {
  font-size: 0.6875rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.status-pill {
  padding: 5px 10px;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: capitalize;
  background: var(--hover-overlay);
  color: var(--text-secondary);
}

.status-pill-backlog,
.status-pill-queued { color: var(--badge-queued-color); }
.status-pill-in_progress { color: var(--badge-in_progress-color); }
.status-pill-done { color: var(--badge-done-color); }
.status-pill-failed { color: var(--badge-failed-color); }

.summary-copy {
  margin: 0;
  font-size: 0.88rem;
  line-height: 1.6;
  color: var(--text-secondary);
}

.summary-metrics {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.metric-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 11px 12px;
  border-radius: var(--radius-xs);
  background: var(--bg-surface);
}

.metric-label,
.signal-label {
  font-size: 0.67rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.metric-value {
  font-size: 0.84rem;
  color: var(--text-primary);
  font-weight: 600;
}

.signal-box {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.signal-content {
  padding: 12px 13px;
  border-radius: var(--radius-xs);
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 0.82rem;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
}

.panel-button {
  width: 100%;
  border: none;
  background: transparent;
  cursor: pointer;
  font: inherit;
}

.panel-meta {
  color: var(--text-muted);
  font-size: 0.78rem;
}

.log-box {
  max-height: 340px;
  overflow-y: auto;
  padding: 0 16px 16px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.7rem;
  line-height: 1.7;
}

.log-line {
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-all;
}

.log-stderr {
  color: #f87171;
}

.live-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success);
  margin-right: 6px;
  animation: pulse-glow 1.6s ease infinite;
}

.cursor {
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
</style>
