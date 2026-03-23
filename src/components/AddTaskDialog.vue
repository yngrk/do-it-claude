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
    <div class="modal">
      <div class="modal-header">
        <h2>Add Task</h2>
        <button class="btn-icon" @click="close">✕</button>
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
            placeholder="Describe what Claude should do..."
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
