<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useProjectStore } from '../stores/projectStore'
import { useTaskStore } from '../stores/taskStore'
import { computed } from 'vue'
import type { Project } from '../types'

const props = defineProps<{ project: Project }>()
const emit = defineEmits<{ deleted: [] }>()

const router = useRouter()
const projectStore = useProjectStore()
const taskStore = useTaskStore()

const taskCounts = computed(() => {
  const counts = { backlog: 0, queued: 0, in_progress: 0, done: 0, failed: 0 }
  taskStore.tasks.forEach(t => {
    if (t.project_id === props.project.id && t.status in counts) {
      counts[t.status]++
    }
  })
  return counts
})

function navigate() {
  router.push(`/project/${props.project.id}`)
}

async function handleDelete(e: MouseEvent) {
  e.stopPropagation()
  if (confirm(`Delete project "${props.project.name}"?`)) {
    await projectStore.deleteProject(props.project.id)
    emit('deleted')
  }
}
</script>

<template>
  <div class="project-card" @click="navigate">
    <div class="project-card-header">
      <h3 class="project-name">{{ project.name }}</h3>
      <button class="btn-icon btn-danger" @click="handleDelete" title="Delete project">✕</button>
    </div>
    <p class="project-path">{{ project.path }}</p>
    <div class="task-counts">
      <span class="badge badge-backlog">{{ taskCounts.backlog }} backlog</span>
      <span class="badge badge-queued">{{ taskCounts.queued }} queued</span>
      <span class="badge badge-in_progress">{{ taskCounts.in_progress }} running</span>
      <span class="badge badge-done">{{ taskCounts.done }} done</span>
      <span class="badge badge-failed">{{ taskCounts.failed }} failed</span>
    </div>
  </div>
</template>
