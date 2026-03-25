import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadProjects() {
    loading.value = true
    error.value = null
    try {
      projects.value = await invoke<Project[]>('get_projects')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function createProject(name: string, path: string) {
    const project = await invoke<Project>('create_project', { name, path })
    projects.value.push(project)
    return project
  }

  async function deleteProject(id: string) {
    await invoke('delete_project', { id })
    projects.value = projects.value.filter(p => p.id !== id)
  }

  async function validateProjectPath(path: string): Promise<boolean> {
    return await invoke<boolean>('validate_project_path', { path })
  }

  async function updateSystemPrompt(projectId: string, systemPrompt: string | null) {
    await invoke('update_project_system_prompt', { id: projectId, system_prompt: systemPrompt })
    const project = projects.value.find(p => p.id === projectId)
    if (project) project.system_prompt = systemPrompt
  }

  async function updateProjectMode(projectId: string, modeId: string | null) {
    await invoke('update_project_mode', { id: projectId, mode_id: modeId })
    const project = projects.value.find(p => p.id === projectId)
    if (project) project.mode_id = modeId
  }

  return { projects, loading, error, loadProjects, createProject, deleteProject, validateProjectPath, updateSystemPrompt, updateProjectMode }
})
