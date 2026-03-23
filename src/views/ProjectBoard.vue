<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useTaskStore } from '../stores/taskStore'
import KanbanColumn from '../components/KanbanColumn.vue'
import TaskDetail from '../components/TaskDetail.vue'
import AddTaskDialog from '../components/AddTaskDialog.vue'
import type { Task } from '../types'

const route = useRoute()
const taskStore = useTaskStore()
const projectId = computed(() => route.params.id as string)
const showAddDialog = ref(false)
const selectedTask = ref<Task | null>(null)
const queueRunning = ref(false)

onMounted(() => {
  taskStore.loadTasks(projectId.value)
})

const columns: { title: string; status: Task['status'] }[] = [
  { title: 'Backlog', status: 'backlog' },
  { title: 'Queued', status: 'queued' },
  { title: 'In Progress', status: 'in_progress' },
  { title: 'Done / Failed', status: 'done' },
]

function getTasksForStatus(status: Task['status']) {
  if (status === 'done') {
    return taskStore.tasks
      .filter(t => t.status === 'done' || t.status === 'failed')
      .sort((a, b) => a.sort_order - b.sort_order)
  }
  return taskStore.tasks
    .filter(t => t.status === status)
    .sort((a, b) => a.sort_order - b.sort_order)
}

async function handleTaskMoved(taskId: string, newStatus: Task['status'], newSortOrder: number) {
  await taskStore.moveTask(taskId, newStatus, newSortOrder)
  if (newStatus === 'queued') {
    await taskStore.startQueue(projectId.value)
    queueRunning.value = true
  }
}

async function toggleQueue() {
  if (queueRunning.value) {
    await taskStore.stopQueue(projectId.value)
    queueRunning.value = false
  } else {
    await taskStore.startQueue(projectId.value)
    queueRunning.value = true
  }
}

function openDetail(task: Task) {
  selectedTask.value = task
}
</script>

<template>
  <div class="project-board">
    <div class="board-header">
      <h1>Project Board</h1>
      <div class="board-actions">
        <button class="btn btn-secondary" @click="toggleQueue">
          {{ queueRunning ? 'Stop Queue' : 'Start Queue' }}
        </button>
        <button class="btn btn-primary" @click="showAddDialog = true">+ Add Task</button>
      </div>
    </div>

    <div v-if="taskStore.loading" class="loading">Loading tasks...</div>
    <div v-else class="kanban-board">
      <KanbanColumn
        v-for="col in columns"
        :key="col.status"
        :title="col.title"
        :status="col.status"
        :tasks="getTasksForStatus(col.status)"
        @task-moved="handleTaskMoved"
        @open-detail="openDetail"
      />
    </div>

    <TaskDetail
      :task="selectedTask"
      @close="selectedTask = null"
      @retry="handleTaskMoved($event, 'queued', 0)"
    />

    <AddTaskDialog
      :visible="showAddDialog"
      :project-id="projectId"
      @close="showAddDialog = false"
    />
  </div>
</template>
