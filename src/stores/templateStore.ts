import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PromptTemplate, ModeFile } from '../types'

export const useTemplateStore = defineStore('template', () => {
  const templates = ref<PromptTemplate[]>([])
  const modeFiles = ref<Record<string, ModeFile[]>>({})
  const loading = ref(false)

  async function loadTemplates() {
    loading.value = true
    try {
      templates.value = await invoke<PromptTemplate[]>('get_templates')
    } finally {
      loading.value = false
    }
  }

  async function createTemplate(name: string, content: string) {
    const template = await invoke<PromptTemplate>('create_template', { name, content })
    templates.value.push(template)
    return template
  }

  async function updateTemplate(id: string, name: string, content: string) {
    const template = await invoke<PromptTemplate>('update_template', { id, name, content })
    const idx = templates.value.findIndex(t => t.id === id)
    if (idx !== -1) templates.value[idx] = template
    return template
  }

  async function deleteTemplate(id: string) {
    await invoke('delete_template', { id })
    templates.value = templates.value.filter(t => t.id !== id)
  }

  async function loadModeFiles(modeId: string) {
    const files = await invoke<ModeFile[]>('get_mode_files', { modeId })
    modeFiles.value[modeId] = files
    return files
  }

  async function setModeFiles(modeId: string, files: { file_path: string; content: string }[]) {
    const result = await invoke<ModeFile[]>('set_mode_files', { modeId, files })
    modeFiles.value[modeId] = result
    return result
  }

  return { templates, modeFiles, loading, loadTemplates, createTemplate, updateTemplate, deleteTemplate, loadModeFiles, setModeFiles }
})
