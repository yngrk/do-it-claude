import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'
import type { Router } from 'vue-router'
import { useNotificationSettings } from '../composables/useNotificationSettings'

type NotificationFocus = 'details' | 'chat'

interface NotificationTarget {
  projectId?: string
  taskId?: string
  focus?: NotificationFocus
}

interface TaskCompletedPayload {
  task_id: string
  project_id: string
  exit_code: number
  status: string
  task_title: string
  project_name: string
}

interface TaskChatCompletedPayload {
  task_id: string
  project_id: string
  task_title: string
  project_name: string
  message: {
    content: string
  }
}

let initialized = false

async function ensurePermission() {
  if (await isPermissionGranted()) return true
  return (await requestPermission()) === 'granted'
}

function shouldSuppress() {
  const { settings } = useNotificationSettings()
  return settings.value.suppress_when_focused
    && document.visibilityState === 'visible'
    && document.hasFocus()
}

async function focusApp() {
  const appWindow = getCurrentWindow()
  try { await appWindow.show() } catch {}
  try { await appWindow.unminimize() } catch {}
  try { await appWindow.setFocus() } catch {}
}

async function navigateToTarget(router: Router, target: NotificationTarget) {
  if (!target.projectId) return

  await focusApp()

  const query: Record<string, string> = {}
  if (target.taskId) query.task = target.taskId
  if (target.focus) query.focus = target.focus

  await router.push({
    path: `/project/${target.projectId}`,
    query,
  })
}

async function showNotification(
  router: Router,
  title: string,
  body: string,
  target?: NotificationTarget,
) {
  const { settings } = useNotificationSettings()
  if (!settings.value.enabled || shouldSuppress()) return
  if (!(await ensurePermission())) return

  const notification = new window.Notification(title, {
    body,
    tag: target?.taskId ? `task-${target.taskId}` : undefined,
  })

  if (target?.projectId) {
    notification.onclick = async (event) => {
      event.preventDefault()
      await navigateToTarget(router, target)
      notification.close()
    }
  }
}

export async function requestNotificationAccess() {
  return ensurePermission()
}

export async function sendTestNotification(router: Router) {
  await showNotification(
    router,
    'Do It Claude',
    'Notifications are enabled.',
  )
}

export async function initNotificationService(router: Router) {
  if (initialized) return
  initialized = true

  const { settings } = useNotificationSettings()

  await listen<TaskCompletedPayload>('task-completed', async (event) => {
    const payload = event.payload
    if (payload.status === 'done' && settings.value.notify_on_task_done) {
      await showNotification(
        router,
        `Task completed: ${payload.task_title}`,
        `${payload.project_name} finished successfully.`,
        { projectId: payload.project_id, taskId: payload.task_id, focus: 'details' },
      )
    }

    if (payload.status === 'failed' && settings.value.notify_on_task_failed) {
      await showNotification(
        router,
        `Task failed: ${payload.task_title}`,
        `${payload.project_name} exited with code ${payload.exit_code}.`,
        { projectId: payload.project_id, taskId: payload.task_id, focus: 'details' },
      )
    }
  })

  await listen<TaskChatCompletedPayload>('task-chat-completed', async (event) => {
    const payload = event.payload
    if (!settings.value.notify_on_chat_reply) return

    const preview = payload.message.content.trim().replace(/\s+/g, ' ').slice(0, 120)
    await showNotification(
      router,
      `Claude replied: ${payload.task_title}`,
      preview || `${payload.project_name} has a new reply.`,
      { projectId: payload.project_id, taskId: payload.task_id, focus: 'chat' },
    )
  })
}
