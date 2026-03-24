<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '../stores/taskStore'
import { TASK_TAGS } from '../types'
import type { TaskTag } from '../types'

const props = defineProps<{ visible: boolean; projectId: string }>()
const emit = defineEmits<{ close: [] }>()

const taskStore = useTaskStore()
const title = ref('')
const description = ref('')
const tag = ref<TaskTag | null>(null)
const submitting = ref(false)

function selectTag(value: TaskTag) {
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
    <div class="modal slide-up">
      <div class="modal-header">
        <h2>Add Task</h2>
        <button class="btn-icon" @click="close" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Title</label>
          <input v-model="title" type="text" placeholder="Task title" class="form-input" />
        </div>
        <div class="form-group">
          <label>Tag</label>
          <div class="tag-picker">
            <button
              v-for="t in TASK_TAGS"
              :key="t.value"
              class="tag-option"
              :class="{ 'tag-selected': tag === t.value }"
              :style="tag === t.value ? { background: t.color + '22', borderColor: t.color + '55', color: t.color } : {}"
              @click="selectTag(t.value)"
              type="button"
            >
              <span class="tag-dot" :style="{ background: t.color }"></span>
              {{ t.label }}
            </button>
          </div>
        </div>
        <div class="form-group">
          <label>Description / Prompt</label>
          <textarea
            v-model="description"
            placeholder="Write the prompt Claude will execute..."
            class="form-input form-textarea"
            rows="5"
          />
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" @click="close">Cancel</button>
        <button
          class="btn btn-primary"
          :disabled="!title || submitting"
          @click="submit"
        >{{ submitting ? 'Creating...' : 'Create' }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tag-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag-option {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 100px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  font-size: 0.75rem;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s ease;
}

.tag-option:hover {
  border-color: var(--border-hover);
  color: var(--text-secondary);
}

.tag-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
</style>
