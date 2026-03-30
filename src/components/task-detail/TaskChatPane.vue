<script setup lang="ts">
import { formatTimestamp } from '../../utils/dateFormat'

defineProps<{
  providerLabel: string
  transcriptEntries: Array<{
    id: string
    role: 'user' | 'assistant' | 'execution'
    content: string
    kind: 'chat' | 'execution'
    created_at: string
    tone?: 'stdout' | 'stderr'
  }>
  chatDraft: string
  isChatSending: boolean
  chatError: string | null
  chatDisabledReason: string | null
  includeExecutionInChat: boolean
  liveOutput: string[]
  chatInput: string
}>()

const emit = defineEmits<{
  'update:includeExecutionInChat': [value: boolean]
  'update:chatInput': [value: string]
  send: []
}>()
</script>

<template>
  <section class="chat-pane">
    <div class="pane-header">
      <div>
        <span class="pane-kicker">Chat</span>
        <p class="pane-caption">Ask follow-up questions, request verification, or plan the next step.</p>
      </div>
      <label class="chat-toggle">
        <input
          :checked="includeExecutionInChat"
          type="checkbox"
          @change="emit('update:includeExecutionInChat', ($event.target as HTMLInputElement).checked)"
        />
        Include execution events
      </label>
    </div>

    <div class="chat-box">
      <div v-if="transcriptEntries.length === 0 && !chatDraft && !isChatSending" class="chat-empty">
        No chat yet. Ask {{ providerLabel }} about this task to start a thread.
      </div>

      <div
        v-for="entry in transcriptEntries"
        :key="entry.id"
        class="chat-message"
        :class="[
          `chat-${entry.role}`,
          entry.kind === 'execution' ? `chat-${entry.tone}` : '',
        ]"
      >
        <div class="chat-meta">
          <span class="chat-role">
            {{ entry.role === 'user' ? 'You' : entry.role === 'assistant' ? providerLabel : 'Execution' }}
          </span>
          <span class="chat-time">{{ formatTimestamp(entry.created_at) }}</span>
        </div>
        <div class="chat-content">{{ entry.content }}</div>
      </div>

      <div v-if="chatDraft" class="chat-message chat-assistant chat-draft">
        <div class="chat-meta">
          <span class="chat-role">{{ providerLabel }}</span>
          <span class="chat-time">typing</span>
        </div>
        <div class="chat-content">{{ chatDraft }}</div>
      </div>

      <div v-else-if="isChatSending" class="chat-message chat-assistant">
        <div class="chat-meta">
          <span class="chat-role">{{ providerLabel }}</span>
          <span class="chat-time">thinking</span>
        </div>
        <div class="chat-thinking">
          <span class="thinking-dot"></span>
          <span class="thinking-dot"></span>
          <span class="thinking-dot"></span>
        </div>
      </div>

      <div
        v-if="includeExecutionInChat && liveOutput.length > 0"
        class="chat-message chat-execution"
      >
        <div class="chat-meta">
          <span class="chat-role">Execution</span>
          <span class="chat-time">live</span>
        </div>
        <div class="chat-content">{{ liveOutput[liveOutput.length - 1] }}</div>
      </div>
    </div>

    <div v-if="chatError" class="chat-error">{{ chatError }}</div>
    <div v-if="chatDisabledReason" class="chat-note">{{ chatDisabledReason }}</div>

    <div class="composer">
      <textarea
        :value="chatInput"
        class="composer-input"
        rows="4"
        :placeholder="`Ask ${providerLabel} about this task...`"
        :disabled="!!chatDisabledReason || isChatSending"
        @input="emit('update:chatInput', ($event.target as HTMLTextAreaElement).value)"
        @keydown.enter.exact.prevent="emit('send')"
      />
      <button
        class="btn btn-primary btn-sm"
        :disabled="!chatInput.trim() || !!chatDisabledReason || isChatSending"
        @click="emit('send')"
      >
        {{ isChatSending ? 'Sending...' : 'Send' }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.chat-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 100%;
  height: 100%;
  overflow: hidden;
}

.pane-header {
  flex-shrink: 0;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.pane-kicker {
  display: block;
  font-size: 0.6875rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.pane-caption {
  margin: 4px 0 0;
  font-size: 0.8rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.chat-toggle {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
}

.chat-toggle input {
  accent-color: var(--accent);
}

.chat-box {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
}

.chat-empty,
.chat-note {
  font-size: 0.78rem;
  color: var(--text-muted);
}

.chat-message {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 12px 13px;
  border-radius: var(--radius-xs);
  border: 1px solid var(--border);
  background: var(--bg-surface);
}

.chat-user {
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-surface));
  border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
}

.chat-execution {
  background: var(--bg-terminal);
}

.chat-stderr {
  border-color: rgba(248, 113, 113, 0.35);
}

.chat-draft {
  border-style: dashed;
}

.chat-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  color: var(--text-muted);
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.chat-role {
  font-weight: 700;
}

.chat-content {
  font-size: 0.84rem;
  line-height: 1.65;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}

.chat-thinking {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 24px;
}

.thinking-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-secondary);
  opacity: 0.45;
  animation: thinking-bounce 1.1s infinite ease-in-out;
}

.thinking-dot:nth-child(2) { animation-delay: 0.15s; }
.thinking-dot:nth-child(3) { animation-delay: 0.3s; }

@keyframes thinking-bounce {
  0%, 80%, 100% { transform: scale(0.8); opacity: 0.35; }
  40% { transform: scale(1); opacity: 0.95; }
}

.chat-error {
  flex-shrink: 0;
  font-size: 0.78rem;
  color: var(--error);
}

.composer {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.composer-input {
  width: 100%;
  min-height: 98px;
  resize: vertical;
  padding: 13px 14px;
  border-radius: var(--radius-xs);
  border: 1px solid var(--border-hover);
  background: var(--bg-surface);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.84rem;
  line-height: 1.6;
  outline: none;
}

.composer-input:focus {
  border-color: var(--accent);
}

.composer-input:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}
</style>
