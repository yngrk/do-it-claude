import { computed, ref, watch } from 'vue'
import type { NotificationSettings } from '../types'

const STORAGE_KEY = 'notification-settings'

const defaultSettings: NotificationSettings = {
  enabled: true,
  notify_on_task_done: true,
  notify_on_task_failed: true,
  notify_on_chat_reply: true,
  suppress_when_focused: true,
}

function loadSettings(): NotificationSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return { ...defaultSettings }
    return { ...defaultSettings, ...JSON.parse(raw) }
  } catch {
    return { ...defaultSettings }
  }
}

const settings = ref<NotificationSettings>(loadSettings())

watch(settings, (value) => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
}, { deep: true })

export function useNotificationSettings() {
  const enabled = computed({
    get: () => settings.value.enabled,
    set: (value: boolean) => {
      settings.value = { ...settings.value, enabled: value }
    },
  })

  function update(patch: Partial<NotificationSettings>) {
    settings.value = { ...settings.value, ...patch }
  }

  function reset() {
    settings.value = { ...defaultSettings }
  }

  return {
    settings,
    enabled,
    update,
    reset,
    defaultSettings,
  }
}
