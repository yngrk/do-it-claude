<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useProjectStore } from '../stores/projectStore'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: []; created: [] }>()

const projectStore = useProjectStore()
const name = ref('')
const path = ref('')
const pathValid = ref<boolean | null>(null)
const pathError = ref('')
const submitting = ref(false)

async function browsePath() {
  const selected = await open({
    directory: true,
    multiple: false,
  })
  if (selected) {
    path.value = selected as string
    await validatePath()
    // Auto-fill name from folder name if empty
    if (!name.value) {
      const parts = path.value.replace(/\/+$/, '').split('/')
      name.value = parts[parts.length - 1] || ''
    }
  }
}

async function validatePath() {
  if (!path.value) {
    pathValid.value = null
    pathError.value = ''
    return
  }
  try {
    pathValid.value = await projectStore.validateProjectPath(path.value)
    pathError.value = pathValid.value ? '' : 'Path is not valid or does not exist'
  } catch {
    pathValid.value = false
    pathError.value = 'Could not validate path'
  }
}

async function submit() {
  if (!name.value || !path.value || pathValid.value === false) return
  submitting.value = true
  try {
    await projectStore.createProject(name.value, path.value)
    name.value = ''
    path.value = ''
    pathValid.value = null
    emit('created')
    emit('close')
  } finally {
    submitting.value = false
  }
}

function close() {
  name.value = ''
  path.value = ''
  pathValid.value = null
  pathError.value = ''
  emit('close')
}
</script>

<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="modal slide-up">
      <div class="modal-header">
        <h2>Add Project</h2>
        <button class="btn-icon" @click="close" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Project folder</label>
          <div class="path-picker" @click="browsePath">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="path-icon">
              <path d="M1.5 3.5C1.5 2.948 1.948 2.5 2.5 2.5H5.5L7 4H11.5C12.052 4 12.5 4.448 12.5 5V10.5C12.5 11.052 12.052 11.5 11.5 11.5H2.5C1.948 11.5 1.5 11.052 1.5 10.5V3.5Z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span v-if="path" class="path-text">{{ path }}</span>
            <span v-else class="path-placeholder">Choose a folder...</span>
          </div>
          <span v-if="pathError" class="field-error">{{ pathError }}</span>
        </div>
        <div class="form-group">
          <label>Name</label>
          <input v-model="name" type="text" placeholder="Project name" class="form-input" />
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" @click="close">Cancel</button>
        <button
          class="btn btn-primary"
          :disabled="!name || !path || pathValid === false || submitting"
          @click="submit"
        >{{ submitting ? 'Creating...' : 'Create' }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.path-picker {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-input);
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.path-picker:hover {
  border-color: var(--border-hover);
}

.path-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.path-text {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.8125rem;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.path-placeholder {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.field-error {
  font-size: 0.75rem;
  color: var(--error);
}
</style>
