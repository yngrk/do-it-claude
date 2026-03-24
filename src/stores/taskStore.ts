import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Task, TaskLog, TaskTag } from '../types'

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const taskLogs = reactive<Record<string, TaskLog[]>>({})
  const liveLogs = ref<Record<string, string[]>>({})

  listen<{ task_id: string }>('task-started', (event) => {
    const taskId = event.payload.task_id
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = { ...tasks.value[idx], status: 'in_progress', started_at: new Date().toISOString() }
      tasks.value = [...tasks.value]
    }
    liveLogs.value = { ...liveLogs.value, [taskId]: [] }
  })

  listen<{ task_id: string; exit_code: number; status: string }>('task-completed', (event) => {
    const { task_id, exit_code, status } = event.payload
    const idx = tasks.value.findIndex(t => t.id === task_id)
    if (idx !== -1) {
      tasks.value[idx] = {
        ...tasks.value[idx],
        status: status as Task['status'],
        exit_code,
        completed_at: new Date().toISOString(),
      }
      tasks.value = [...tasks.value]
    }
  })

  listen<{ task_id: string; content: string; log_type: 'stdout' | 'stderr' }>('task-output', (event) => {
    const { task_id, content } = event.payload
    const existing = liveLogs.value[task_id] || []
    liveLogs.value = {
      ...liveLogs.value,
      [task_id]: [...existing, content],
    }
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

  async function createTask(projectId: string, title: string, description: string, tag: TaskTag | null = null) {
    const task = await invoke<Task>('create_task', { projectId, title, description, tag })
    tasks.value = [...tasks.value, task]
    return task
  }

  async function updateTask(taskId: string, updates: { title?: string; description?: string; tag?: string | null }) {
    const task = await invoke<Task>('update_task', { id: taskId, ...updates })
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = task
      tasks.value = [...tasks.value]
    }
    return task
  }

  async function deleteTask(taskId: string) {
    await invoke('delete_task', { id: taskId })
    tasks.value = tasks.value.filter(t => t.id !== taskId)
  }

  async function moveTask(taskId: string, newStatus: Task['status'], newSortOrder: number) {
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = { ...tasks.value[idx], status: newStatus, sort_order: newSortOrder }
      tasks.value = [...tasks.value]
    }
    try {
      await invoke('move_task', { id: taskId, newStatus, newSortOrder })
    } catch (e) {
      console.error('move_task failed:', e)
    }
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

  async function pauseQueue(projectId: string) {
    await invoke('pause_queue', { projectId })
  }

  async function cancelAndRevert(projectId: string) {
    await invoke('cancel_and_revert', { projectId })
  }

  async function resetSession(projectId: string) {
    await invoke('reset_session', { projectId })
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
    pauseQueue,
    cancelAndRevert,
    resetSession,
  }
})
