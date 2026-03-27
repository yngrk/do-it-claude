import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { ProjectMessage } from '../types'

export const useChatStore = defineStore('chat', () => {
  const messages = reactive<Record<string, ProjectMessage[]>>({})
  const liveDraft = ref<Record<string, string>>({})
  const sending = ref<Record<string, boolean>>({})
  const errors = ref<Record<string, string | null>>({})

  listen<{ project_id: string }>('project-chat-started', (event) => {
    const pid = event.payload.project_id
    liveDraft.value = { ...liveDraft.value, [pid]: '' }
    sending.value = { ...sending.value, [pid]: true }
    errors.value = { ...errors.value, [pid]: null }
  })

  listen<{ project_id: string; content: string }>('project-chat-chunk', (event) => {
    const { project_id, content } = event.payload
    liveDraft.value = {
      ...liveDraft.value,
      [project_id]: (liveDraft.value[project_id] || '') + content,
    }
  })

  listen<{ project_id: string; message: ProjectMessage }>('project-chat-completed', (event) => {
    const { project_id, message } = event.payload
    messages[project_id] = [...(messages[project_id] || []), message]
    liveDraft.value = { ...liveDraft.value, [project_id]: '' }
    sending.value = { ...sending.value, [project_id]: false }
  })

  listen<{ project_id: string; error: string }>('project-chat-failed', (event) => {
    const { project_id, error } = event.payload
    liveDraft.value = { ...liveDraft.value, [project_id]: '' }
    sending.value = { ...sending.value, [project_id]: false }
    errors.value = { ...errors.value, [project_id]: error }
  })

  async function loadMessages(projectId: string) {
    const msgs = await invoke<ProjectMessage[]>('get_project_messages', { projectId })
    messages[projectId] = msgs
  }

  async function sendMessage(projectId: string, content: string, attachments: { data: string; media_type: string; name: string }[] = []) {
    const trimmed = content.trim()
    if (!trimmed && attachments.length === 0) return

    // Optimistic user message
    const optimistic: ProjectMessage = {
      id: `local-${crypto.randomUUID()}`,
      project_id: projectId,
      role: 'user',
      content: trimmed,
      created_at: new Date().toISOString(),
    }
    messages[projectId] = [...(messages[projectId] || []), optimistic]
    errors.value = { ...errors.value, [projectId]: null }

    try {
      return await invoke<ProjectMessage>('send_project_message', {
        projectId,
        content: trimmed,
        attachments: attachments.length > 0 ? attachments : null
      })
    } catch (e) {
      messages[projectId] = (messages[projectId] || []).filter(m => m.id !== optimistic.id)
      sending.value = { ...sending.value, [projectId]: false }
      errors.value = { ...errors.value, [projectId]: String(e) }
      throw e
    }
  }

  async function clearChat(projectId: string) {
    await invoke('clear_project_chat', { projectId })
    messages[projectId] = []
    liveDraft.value = { ...liveDraft.value, [projectId]: '' }
    sending.value = { ...sending.value, [projectId]: false }
    errors.value = { ...errors.value, [projectId]: null }
  }

  return {
    messages,
    liveDraft,
    sending,
    errors,
    loadMessages,
    sendMessage,
    clearChat,
  }
})
