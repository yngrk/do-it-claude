<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '../stores/taskStore'

const props = defineProps<{ visible: boolean; projectId: string }>()
const emit = defineEmits<{ close: [] }>()

const taskStore = useTaskStore()
const title = ref('')
const description = ref('')
const submitting = ref(false)

async function submit() {
  if (!title.value) return
  submitting.value = true
  try {
    await taskStore.createTask(props.projectId, title.value, description.value)
    title.value = ''
    description.value = ''
    emit('close')
  } finally {
    submitting.value = false
  }
}

function close() {
  title.value = ''
  description.value = ''
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
