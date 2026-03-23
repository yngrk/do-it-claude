<script setup lang="ts">
import { ref } from 'vue'
import { useProjectStore } from '../stores/projectStore'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const projectStore = useProjectStore()
const name = ref('')
const path = ref('')
const pathValid = ref<boolean | null>(null)
const pathError = ref('')
const submitting = ref(false)

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
    <div class="modal">
      <div class="modal-header">
        <h2>Add Project</h2>
        <button class="btn-icon" @click="close">✕</button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Name</label>
          <input v-model="name" type="text" placeholder="My Project" class="form-input" />
        </div>
        <div class="form-group">
          <label>Path</label>
          <input
            v-model="path"
            type="text"
            placeholder="/path/to/project"
            class="form-input"
            :class="{ 'input-error': pathValid === false, 'input-success': pathValid === true }"
            @blur="validatePath"
          />
          <span v-if="pathError" class="field-error">{{ pathError }}</span>
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
