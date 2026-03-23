import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Task, TaskLog } from '../types'

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const taskLogs = reactive<Record<string, TaskLog[]>>({})
  const liveLogs = reactive<Record<string, string[]>>({})

  listen<{ task_id: string }>('task-started', (event) => {
    const taskId = event.payload.task_id
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = 'in_progress'
    }
    if (!liveLogs[taskId]) {
      liveLogs[taskId] = []
    }
  })

  listen<{ task_id: string; exit_code: number; status: string }>('task-completed', (event) => {
    const { task_id, exit_code, status } = event.payload
    const task = tasks.value.find(t => t.id === task_id)
    if (task) {
      task.status = status as Task['status']
      task.exit_code = exit_code
      task.completed_at = new Date().toISOString()
    }
  })

  listen<{ task_id: string; content: string; log_type: 'stdout' | 'stderr' }>('task-output', (event) => {
    const { task_id, content } = event.payload
    if (!liveLogs[task_id]) {
      liveLogs[task_id] = []
    }
    liveLogs[task_id].push(content)
  })

  async function loadTasks(projectId: string) {
    loading.value = true
    error.value = null
    try {
      tasks.value = await invoke<Task[]>('get_tasks', { projectId })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function createTask(projectId: string, title: string, description: string) {
    const task = await invoke<Task>('create_task', { projectId, title, description })
    tasks.value.push(task)
    return task
  }

  async function updateTask(taskId: string, updates: Partial<Task>) {
    const task = await invoke<Task>('update_task', { taskId, ...updates })
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = task
    }
    return task
  }

  async function deleteTask(taskId: string) {
    await invoke('delete_task', { taskId })
    tasks.value = tasks.value.filter(t => t.id !== taskId)
  }

  async function moveTask(taskId: string, newStatus: Task['status'], newSortOrder: number) {
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = newStatus
      task.sort_order = newSortOrder
    }
    await invoke('move_task', { taskId, newStatus, newSortOrder })
  }

  async function loadTaskLogs(taskId: string) {
    const logs = await invoke<TaskLog[]>('get_task_logs', { taskId })
    taskLogs[taskId] = logs
    return logs
  }

  async function startQueue(projectId: string) {
    await invoke('start_queue', { projectId })
  }

  async function stopQueue(projectId: string) {
    await invoke('stop_queue', { projectId })
  }

  return {
    tasks,
    loading,
    error,
    taskLogs,
    liveLogs,
    loadTasks,
    createTask,
    updateTask,
    deleteTask,
    moveTask,
    loadTaskLogs,
    startQueue,
    stopQueue,
  }
})
