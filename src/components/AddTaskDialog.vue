<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '../stores/taskStore'
import { DEFAULT_TASK_TAGS } from '../types'

const props = defineProps<{ visible: boolean; projectId: string }>()
const emit = defineEmits<{ close: [] }>()

const taskStore = useTaskStore()
const title = ref('')
const description = ref('')
const tag = ref<string | null>(null)
const submitting = ref(false)

function selectTag(value: string) {
  tag.value = tag.value === value ? null : value
}

async function submit() {
  if (!title.value) return
  submitting.value = true
  try {
    await taskStore.createTask(props.projectId, title.value, description.value, tag.value)
    title.value = ''
    description.value = ''
    tag.value = null
    emit('close')
  } finally {
    submitting.value = false
  }
}

function close() {
  title.value = ''
  description.value = ''
  tag.value = null
  emit('close')
}
</script>

<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="modal td-modal slide-up">
      <!-- Header -->
      <div class="td-header">
        <span class="td-header-title">New Task</span>
        <button class="btn-icon" @click="close" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="td-body">
        <!-- Title -->
        <div class="td-section">
          <span class="td-section-label">Title</span>
          <input
            v-model="title"
            type="text"
            class="td-field-input"
            placeholder="Task title..."
          />
        </div>

        <!-- Labels -->
        <div class="td-labels">
          <button
            v-for="t in DEFAULT_TASK_TAGS"
            :key="t.value"
            class="td-label-chip"
            :class="{ 'td-label-active': tag === t.value }"
            :style="tag === t.value
              ? { background: t.color + '30', borderColor: t.color + '66', color: t.color }
              : { borderColor: t.color + '33', color: t.color + 'aa' }"
            @click="selectTag(t.value)"
            type="button"
          >
            <span class="td-label-dot" :style="{ background: t.color }"></span>
            {{ t.label }}
          </button>
        </div>

        <!-- Prompt -->
        <div class="td-section">
          <span class="td-section-label">Prompt</span>
          <textarea
            v-model="description"
            placeholder="Write the prompt Claude will execute..."
            class="td-prompt-edit"
            rows="6"
          />
        </div>
      </div>

      <!-- Footer -->
      <div class="td-footer">
        <button class="btn btn-secondary btn-sm" @click="close">Cancel</button>
        <button
          class="btn btn-primary btn-sm"
          :disabled="!title || submitting"
          @click="submit"
        >{{ submitting ? 'Creating...' : 'Create' }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.td-modal { width: 560px; }

.td-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.td-header-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
}

.td-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.td-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.td-section-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-secondary);
}

.td-field-input {
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-weight: 500;
  font-family: inherit;
  outline: none;
  width: 100%;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.td-field-input:focus { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-glow); }
.td-field-input::placeholder { color: var(--text-secondary); }

.td-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.td-label-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 100px;
  border: 1px solid;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s ease;
  background: transparent;
}

.td-label-chip:not(.td-label-active) { opacity: 0.7; }
.td-label-chip:not(.td-label-active):hover { opacity: 1; }
.td-label-active { opacity: 1; }

.td-label-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.td-prompt-edit {
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: inherit;
  line-height: 1.65;
  outline: none;
  resize: vertical;
  min-height: 100px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.td-prompt-edit:focus { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-glow); }
.td-prompt-edit::placeholder { color: var(--text-secondary); }

.td-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 20px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}
</style>
