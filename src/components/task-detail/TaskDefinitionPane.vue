<script setup lang="ts">
import { computed } from 'vue'
import { DEFAULT_TASK_TAGS } from '../../types'
import type { Task, TokenEstimate } from '../../types'

const props = defineProps<{
  task: Task
  isEditable: boolean
  providerLabel: string
  editTitle: string
  editDescription: string
  editTag: string | null
  editModel: string | null
  modelOptions: { value: string; label: string }[]
  tokenEstimate: TokenEstimate | null
}>()

const emit = defineEmits<{
  'update:title': [value: string]
  'update:description': [value: string]
  'update:tag': [value: string | null]
  'update:model': [value: string | null]
  'save-model': []
}>()

const activeTag = computed(() => {
  const value = props.isEditable ? props.editTag : props.task.tag
  return DEFAULT_TASK_TAGS.find(tag => tag.value === value) ?? null
})

function selectTag(value: string) {
  emit('update:tag', props.editTag === value ? null : value)
}
</script>

<template>
  <section class="definition-pane">
    <div class="pane-header">
      <span class="pane-kicker">Definition</span>
      <span class="pane-caption">{{ isEditable ? 'Edit what the AI should do next.' : 'Review the exact task specification.' }}</span>
    </div>

    <div class="field-block">
      <span class="field-label">Title</span>
      <input
        v-if="isEditable"
        :value="editTitle"
        type="text"
        class="field-input"
        placeholder="Task title..."
        @input="emit('update:title', ($event.target as HTMLInputElement).value)"
      />
      <div v-else class="field-readonly field-title">{{ task.title }}</div>
    </div>

    <div class="field-block">
      <span class="field-label">Tag</span>
      <div v-if="isEditable" class="tag-row">
        <button
          v-for="tag in DEFAULT_TASK_TAGS"
          :key="tag.value"
          type="button"
          class="tag-chip"
          :class="{ 'tag-chip-active': editTag === tag.value }"
          :style="editTag === tag.value
            ? { background: `${tag.color}20`, borderColor: `${tag.color}55`, color: tag.color }
            : { borderColor: `${tag.color}22`, color: `${tag.color}bb` }"
          @click="selectTag(tag.value)"
        >
          <span class="tag-dot" :style="{ background: tag.color }"></span>
          {{ tag.label }}
        </button>
      </div>
      <div v-else class="field-readonly">
        <span
          v-if="activeTag"
          class="tag-chip tag-chip-active"
          :style="{ background: `${activeTag.color}20`, borderColor: `${activeTag.color}55`, color: activeTag.color }"
        >
          <span class="tag-dot" :style="{ background: activeTag.color }"></span>
          {{ activeTag.label }}
        </span>
        <span v-else class="muted-copy">No tag</span>
      </div>
    </div>

    <div class="field-block">
      <span class="field-label">Prompt</span>
      <textarea
        v-if="isEditable"
        :value="editDescription"
        class="prompt-input"
        :placeholder="`Write the prompt ${providerLabel} will execute...`"
        rows="12"
        @input="emit('update:description', ($event.target as HTMLTextAreaElement).value)"
      />
      <div v-else class="field-readonly prompt-readonly">{{ task.description || 'No prompt provided.' }}</div>
    </div>

    <div class="field-block">
      <span class="field-label">Model</span>
      <select
        v-if="isEditable"
        :value="editModel ?? ''"
        class="model-select"
        @change="
          emit('update:model', (($event.target as HTMLSelectElement).value || null));
          emit('save-model')
        "
      >
        <option v-for="opt in modelOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
      </select>
      <div v-else class="field-readonly">
        {{ modelOptions.find(option => option.value === task.model)?.label ?? task.model ?? 'Auto' }}
      </div>
    </div>

    <div v-if="tokenEstimate" class="token-box">
      <span class="token-title">Estimated Input</span>
      <span class="token-total">~{{ tokenEstimate.total_tokens.toLocaleString() }} tokens</span>
      <span class="token-breakdown">Prompt {{ tokenEstimate.prompt_tokens }} · System {{ tokenEstimate.system_tokens }}</span>
    </div>
  </section>
</template>

<style scoped>
.definition-pane {
  display: flex;
  flex-direction: column;
  gap: 18px;
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
  font-size: 0.8rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.field-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-label {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.field-input,
.prompt-input,
.model-select {
  width: 100%;
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  color: var(--text-primary);
  font: inherit;
  outline: none;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.field-input {
  padding: 10px 12px;
  font-size: 0.9rem;
  font-weight: 600;
}

.prompt-input {
  min-height: 220px;
  resize: vertical;
  padding: 14px 16px;
  line-height: 1.65;
  font-size: 0.84rem;
}

.model-select {
  padding: 9px 12px;
  cursor: pointer;
}

.field-input:focus,
.prompt-input:focus,
.model-select:focus {
  border-color: var(--accent);
}

.field-readonly {
  color: var(--text-secondary);
  font-size: 0.84rem;
  line-height: 1.65;
}

.field-title {
  color: var(--text-primary);
  font-size: 1rem;
  font-weight: 600;
}

.prompt-readonly {
  min-height: 220px;
  padding: 14px 16px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  white-space: pre-wrap;
  word-break: break-word;
}

.tag-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 11px;
  border-radius: 999px;
  border: 1px solid;
  background: transparent;
  font: inherit;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}

.tag-chip-active {
  opacity: 1;
}

.tag-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.muted-copy {
  color: var(--text-muted);
}

.token-box {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 12px 14px;
  border-radius: var(--radius-xs);
  border: 1px solid var(--border);
  background: var(--bg-elevated);
}

.token-title {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.token-total {
  color: var(--text-primary);
  font-size: 0.88rem;
  font-weight: 600;
}

.token-breakdown {
  color: var(--text-muted);
  font-size: 0.75rem;
}
</style>
