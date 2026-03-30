import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Task, TaskLog, TaskMessage, TaskTag, TokenEstimate } from '../types'

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const taskLogs = reactive<Record<string, TaskLog[]>>({})
  const taskMessages = reactive<Record<string, TaskMessage[]>>({})
  const liveLogs = ref<Record<string, string[]>>({})
  const liveChatDrafts = ref<Record<string, string>>({})
  const chatSending = ref<Record<string, boolean>>({})
  const chatErrors = ref<Record<string, string | null>>({})

  listen<{ project_id: string; tasks: Task[] }>('tasks-created', (event) => {
    const newTasks = event.payload.tasks
    if (newTasks.length > 0) {
      tasks.value = [...tasks.value, ...newTasks]
    }
  })

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

  listen<{ task_id: string }>('task-chat-started', (event) => {
    const taskId = event.payload.task_id
    liveChatDrafts.value = {
      ...liveChatDrafts.value,
      [taskId]: '',
    }
    chatSending.value = {
      ...chatSending.value,
      [taskId]: true,
    }
    chatErrors.value = {
      ...chatErrors.value,
      [taskId]: null,
    }
  })

  listen<{ task_id: string; content: string }>('task-chat-chunk', (event) => {
    const { task_id, content } = event.payload
    liveChatDrafts.value = {
      ...liveChatDrafts.value,
      [task_id]: (liveChatDrafts.value[task_id] || '') + content,
    }
  })

  listen<{ task_id: string; message: TaskMessage }>('task-chat-completed', (event) => {
    const { task_id, message } = event.payload
    taskMessages[task_id] = [...(taskMessages[task_id] || []), message]
    liveChatDrafts.value = {
      ...liveChatDrafts.value,
      [task_id]: '',
    }
    chatSending.value = {
      ...chatSending.value,
      [task_id]: false,
    }
  })

  listen<{ task_id: string; error: string }>('task-chat-failed', (event) => {
    const { task_id, error } = event.payload
    liveChatDrafts.value = {
      ...liveChatDrafts.value,
      [task_id]: '',
    }
    chatSending.value = {
      ...chatSending.value,
      [task_id]: false,
    }
    chatErrors.value = {
      ...chatErrors.value,
      [task_id]: error,
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

  async function createTask(projectId: string, title: string, description: string, tag: TaskTag | null = null, model: string | null = null) {
    const task = await invoke<Task>('create_task', { projectId, title, description, tag, model })
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

  async function loadTaskMessages(taskId: string) {
    const messages = await invoke<TaskMessage[]>('get_task_messages', { taskId })
    taskMessages[taskId] = messages
    return messages
  }

  async function sendTaskMessage(taskId: string, content: string) {
    const trimmed = content.trim()
    if (!trimmed) return

    const optimisticMessage: TaskMessage = {
      id: `local-${crypto.randomUUID()}`,
      task_id: taskId,
      role: 'user',
      content: trimmed,
      message_type: 'chat',
      created_at: new Date().toISOString(),
    }

    taskMessages[taskId] = [...(taskMessages[taskId] || []), optimisticMessage]
    chatErrors.value = {
      ...chatErrors.value,
      [taskId]: null,
    }

    try {
      return await invoke<TaskMessage>('send_task_message', { taskId, content: trimmed })
    } catch (e) {
      taskMessages[taskId] = (taskMessages[taskId] || []).filter(message => message.id !== optimisticMessage.id)
      chatSending.value = {
        ...chatSending.value,
        [taskId]: false,
      }
      chatErrors.value = {
        ...chatErrors.value,
        [taskId]: String(e),
      }
      throw e
    }
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

  async function updateTaskMaxTurns(taskId: string, maxTurns: number | null) {
    const updated = await invoke<Task>('update_task_max_turns', { id: taskId, maxTurns })
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) tasks.value[idx] = updated
    return updated
  }

  async function updateTaskModel(taskId: string, model: string | null) {
    const updated = await invoke<Task>('update_task_model', { id: taskId, model })
    const idx = tasks.value.findIndex(t => t.id === taskId)
    if (idx !== -1) tasks.value[idx] = updated
    return updated
  }

  async function estimateTaskTurns(description: string, tag: string | null): Promise<number> {
    return await invoke<number>('estimate_task_turns', { description, tag })
  }

  async function estimateTaskTokens(taskId: string): Promise<TokenEstimate> {
    return await invoke<TokenEstimate>('estimate_task_tokens', { taskId })
  }

  return {
    tasks,
    loading,
    error,
    taskLogs,
    taskMessages,
    liveLogs,
    liveChatDrafts,
    chatSending,
    chatErrors,
    loadTasks,
    createTask,
    updateTask,
    deleteTask,
    moveTask,
    loadTaskLogs,
    loadTaskMessages,
    sendTaskMessage,
    startQueue,
    stopQueue,
    pauseQueue,
    cancelAndRevert,
    resetSession,
    updateTaskMaxTurns,
    updateTaskModel,
    estimateTaskTurns,
    estimateTaskTokens,
  }
})
