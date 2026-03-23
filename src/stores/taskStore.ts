import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Task, TaskLog } from '../types'

export interface TaskStats {
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_creation_tokens: number
  cost_usd: number
  duration_ms: number
  num_turns: number
  tasks_completed: number
  tasks_failed: number
}

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const taskLogs = reactive<Record<string, TaskLog[]>>({})
  const liveLogs = ref<Record<string, string[]>>({})
  const taskStats = ref<Record<string, TaskStats>>({})
  const totalStats = ref<TaskStats>({
    input_tokens: 0, output_tokens: 0, cache_read_tokens: 0,
    cache_creation_tokens: 0, cost_usd: 0, duration_ms: 0, num_turns: 0,
    tasks_completed: 0, tasks_failed: 0,
  })
  const currentProjectPath = ref<string | null>(null)

  listen<{ task_id: string }>('task-started', (event) => {
    const taskId = event.payload.task_id
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = { ...tasks.value[idx], status: 'in_progress', started_at: new Date().toISOString() }
      tasks.value = [...tasks.value]
    }
    liveLogs.value = { ...liveLogs.value, [taskId]: [] }
  })

  function saveStats() {
    if (currentProjectPath.value) {
      invoke('save_project_stats', {
        path: currentProjectPath.value,
        stats: totalStats.value,
      }).catch(() => {})
    }
  }

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
    if (status === 'done') {
      totalStats.value = { ...totalStats.value, tasks_completed: totalStats.value.tasks_completed + 1 }
    } else if (status === 'failed') {
      totalStats.value = { ...totalStats.value, tasks_failed: totalStats.value.tasks_failed + 1 }
    }
    saveStats()
  })

  listen<{ task_id: string; content: string; log_type: 'stdout' | 'stderr' }>('task-output', (event) => {
    const { task_id, content } = event.payload
    const existing = liveLogs.value[task_id] || []
    liveLogs.value = {
      ...liveLogs.value,
      [task_id]: [...existing, content],
    }
  })

  listen<{
    task_id: string; input_tokens: number; output_tokens: number;
    cache_read_tokens: number; cache_creation_tokens: number;
    cost_usd: number; duration_ms: number; num_turns: number;
  }>('task-stats', (event) => {
    const s = event.payload
    taskStats.value = { ...taskStats.value, [s.task_id]: { ...s, tasks_completed: 0, tasks_failed: 0 } }
    totalStats.value = {
      ...totalStats.value,
      input_tokens: totalStats.value.input_tokens + s.input_tokens,
      output_tokens: totalStats.value.output_tokens + s.output_tokens,
      cache_read_tokens: totalStats.value.cache_read_tokens + s.cache_read_tokens,
      cache_creation_tokens: totalStats.value.cache_creation_tokens + s.cache_creation_tokens,
      cost_usd: totalStats.value.cost_usd + s.cost_usd,
      duration_ms: totalStats.value.duration_ms + s.duration_ms,
      num_turns: totalStats.value.num_turns + s.num_turns,
    }
    saveStats()
  })

  async function loadProjectStats(projectPath: string) {
    currentProjectPath.value = projectPath
    liveLogs.value = {}
    taskStats.value = {}
    const empty: TaskStats = {
      input_tokens: 0, output_tokens: 0, cache_read_tokens: 0,
      cache_creation_tokens: 0, cost_usd: 0, duration_ms: 0, num_turns: 0,
      tasks_completed: 0, tasks_failed: 0,
    }
    try {
      const stats = await invoke<Partial<TaskStats>>('load_project_stats', { path: projectPath })
      totalStats.value = { ...empty, ...stats }
    } catch {
      totalStats.value = empty
    }
  }

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
    tasks.value = [...tasks.value, task]
    return task
  }

  async function updateTask(taskId: string, updates: Partial<Task>) {
    const task = await invoke<Task>('update_task', { taskId, ...updates })
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) {
      tasks.value[idx] = task
      tasks.value = [...tasks.value]
    }
    return task
  }

  async function deleteTask(taskId: string) {
    await invoke('delete_task', { taskId })
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
    taskStats,
    totalStats,
    loadProjectStats,
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
