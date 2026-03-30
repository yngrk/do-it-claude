<script setup lang="ts">
import { formatDate } from '../../utils/dateFormat'
import type { Task } from '../../types'

defineProps<{
  task: Task
  providerLabel: string
  duration: string | null
}>()
</script>

<template>
  <section class="metadata-pane">
    <div class="pane-header">
      <span class="pane-kicker">Details</span>
      <p class="pane-caption">Lifecycle, provider, and execution audit for this task.</p>
    </div>

    <div class="meta-grid">
      <div class="meta-item">
        <span class="meta-label">Status</span>
        <span class="meta-value">{{ task.status.replace('_', ' ') }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Provider</span>
        <span class="meta-value">{{ task.provider === 'codex' ? 'Codex' : task.provider === 'claude' ? 'Claude' : providerLabel }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Model</span>
        <span class="meta-value">{{ task.model || 'Auto' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Duration</span>
        <span class="meta-value">{{ duration || 'Not finished' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Created</span>
        <span class="meta-value">{{ formatDate(task.created_at) || 'Unknown' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Started</span>
        <span class="meta-value">{{ formatDate(task.started_at) || 'Not started' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Completed</span>
        <span class="meta-value">{{ formatDate(task.completed_at) || 'Not completed' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Exit code</span>
        <span class="meta-value">{{ task.exit_code ?? 'Running' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Input tokens</span>
        <span class="meta-value">{{ task.input_tokens?.toLocaleString() || 'Not recorded' }}</span>
      </div>
      <div class="meta-item">
        <span class="meta-label">Output tokens</span>
        <span class="meta-value">{{ task.output_tokens?.toLocaleString() || 'Not recorded' }}</span>
      </div>
    </div>

    <div class="prompt-snapshot">
      <span class="meta-label">Prompt snapshot</span>
      <div class="prompt-body">{{ task.description || 'No prompt provided.' }}</div>
    </div>
  </section>
</template>

<style scoped>
.metadata-pane {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.pane-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pane-kicker {
  font-size: 0.6875rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.pane-caption {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.meta-item,
.prompt-snapshot {
  padding: 12px 13px;
  border-radius: var(--radius-xs);
  border: 1px solid var(--border);
  background: var(--bg-elevated);
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.meta-label {
  font-size: 0.67rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.meta-value {
  font-size: 0.84rem;
  font-weight: 600;
  color: var(--text-primary);
}

.prompt-snapshot {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.prompt-body {
  font-size: 0.83rem;
  line-height: 1.65;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
