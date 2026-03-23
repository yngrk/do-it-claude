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
  <div class="project-card slide-up" @click="navigate">
    <div class="project-card-header">
      <h3 class="project-name">{{ project.name }}</h3>
      <button class="btn-icon btn-danger delete-btn" @click="handleDelete" title="Delete project">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1L11 11M11 1L1 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <div class="project-path-row">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg" class="path-icon">
        <path d="M1.5 3.5C1.5 2.948 1.948 2.5 2.5 2.5H5.5L7 4H11.5C12.052 4 12.5 4.448 12.5 5V10.5C12.5 11.052 12.052 11.5 11.5 11.5H2.5C1.948 11.5 1.5 11.052 1.5 10.5V3.5Z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <p class="project-path">{{ project.path }}</p>
    </div>
    <div class="task-counts">
      <span class="badge badge-backlog"><span class="badge-dot dot-backlog"></span>{{ taskCounts.backlog }} backlog</span>
      <span class="badge badge-queued"><span class="badge-dot dot-queued"></span>{{ taskCounts.queued }} queued</span>
      <span class="badge badge-in_progress"><span class="badge-dot dot-in_progress"></span>{{ taskCounts.in_progress }} running</span>
      <span class="badge badge-done"><span class="badge-dot dot-done"></span>{{ taskCounts.done }} done</span>
      <span class="badge badge-failed"><span class="badge-dot dot-failed"></span>{{ taskCounts.failed }} failed</span>
    </div>
  </div>
</template>

<style scoped>
.delete-btn {
  opacity: 0;
  transition: opacity 0.2s ease;
}

.project-card:hover .delete-btn {
  opacity: 1;
}

.project-path-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 12px;
}

.path-icon {
  color: var(--text-muted, #6b7280);
  flex-shrink: 0;
}

.badge-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-right: 4px;
  flex-shrink: 0;
}

.dot-backlog { background-color: #777; }
.dot-queued { background-color: #3b82f6; }
.dot-in_progress { background-color: #eab308; }
.dot-done { background-color: #22c55e; }
.dot-failed { background-color: #f87171; }
</style>
