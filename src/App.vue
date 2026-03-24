<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { RouterView, useRouter, useRoute } from 'vue-router'
import { useProjectStore } from './stores/projectStore'
import AddProjectDialog from './components/AddProjectDialog.vue'
import BottomPanel from './components/BottomPanel.vue'

const router = useRouter()
const route = useRoute()
const projectStore = useProjectStore()
const showAddDialog = ref(false)
const bottomTab = ref<'logs' | 'terminal' | null>(null)

function toggleBottomTab(tab: 'logs' | 'terminal') {
  bottomTab.value = bottomTab.value === tab ? null : tab
}

onMounted(() => {
  projectStore.loadProjects()
})

const activeProjectId = computed(() => {
  if (route.path.startsWith('/project/')) {
    return route.params.id as string
  }
  return null
})

const isOnSettings = computed(() => route.path === '/settings')

function toggleSettings() {
  if (isOnSettings.value) {
    router.push('/')
  } else {
    router.push('/settings')
  }
}

function selectProject(id: string) {
  router.push(`/project/${id}`)
}

async function closeProject(id: string, e: MouseEvent) {
  e.stopPropagation()
  // Navigate away if closing the active tab
  if (activeProjectId.value === id) {
    const remaining = projectStore.projects.filter(p => p.id !== id)
    if (remaining.length > 0) {
      router.push(`/project/${remaining[0].id}`)
    } else {
      router.push('/')
    }
  }
  await projectStore.deleteProject(id)
}

const activeProject = computed(() => {
  if (!activeProjectId.value) return null
  return projectStore.projects.find(p => p.id === activeProjectId.value) ?? null
})

async function handleProjectCreated() {
  showAddDialog.value = false
  const latest = projectStore.projects[projectStore.projects.length - 1]
  if (latest) {
    router.push(`/project/${latest.id}`)
  }
}

</script>

<template>
  <div class="app-layout">
    <div v-if="!isOnSettings" class="tab-bar" data-tauri-drag-region>
      <!-- New project button -->
      <button class="tab-action tab-add" @click="showAddDialog = true" title="New project">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M7 1.5V12.5M1.5 7H12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>

      <!-- Project tabs -->
      <div class="tab-list">
        <button
          v-for="project in projectStore.projects"
          :key="project.id"
          class="tab"
          :class="{ 'tab-active': activeProjectId === project.id }"
          @click="selectProject(project.id)"
        >
          <span class="tab-label">{{ project.name }}</span>
          <span class="tab-close" role="button" @click="closeProject(project.id, $event)" title="Close tab">
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M2 2L8 8M8 2L2 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            </svg>
          </span>
        </button>
      </div>

    </div>

    <main class="main-content">
      <RouterView />
    </main>

    <BottomPanel
      v-if="bottomTab === 'terminal'"
      :cwd="activeProject?.path ?? null"
    />

    <div class="status-bar">
      <div class="status-left">
        <span v-if="activeProject" class="status-path">{{ activeProject.path }}</span>
      </div>
      <div class="status-right">
        <button class="status-btn" :class="{ 'status-btn-active': bottomTab === 'terminal' }" @click="toggleBottomTab('terminal')" title="Toggle terminal">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2 3L5 6L2 9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M6.5 9H10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
          Terminal
        </button>
        <button class="status-btn" :class="{ 'status-btn-active': isOnSettings }" @click="toggleSettings" title="Settings">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M6 7.5C6.82843 7.5 7.5 6.82843 7.5 6C7.5 5.17157 6.82843 4.5 6 4.5C5.17157 4.5 4.5 5.17157 4.5 6C4.5 6.82843 5.17157 7.5 6 7.5Z" stroke="currentColor" stroke-width="1"/>
            <path d="M9.79 7.6C9.71 7.76 9.71 7.96 9.81 8.12L10.07 8.54C10.18 8.74 10.14 8.99 9.97 9.13L9.13 9.82C8.97 9.95 8.74 9.95 8.58 9.83L8.15 9.54C7.99 9.44 7.81 9.41 7.64 9.48C7.47 9.55 7.3 9.61 7.12 9.65C6.95 9.7 6.81 9.84 6.78 10.03L6.7 10.53C6.66 10.75 6.47 10.92 6.24 10.92H5.17C4.94 10.92 4.75 10.75 4.71 10.53L4.63 10.03C4.6 9.84 4.46 9.7 4.29 9.65C4.11 9.61 3.94 9.55 3.77 9.48C3.6 9.41 3.41 9.44 3.26 9.54L2.83 9.83C2.67 9.95 2.44 9.95 2.28 9.82L1.44 9.13C1.27 9 1.22 8.74 1.34 8.54L1.6 8.12C1.7 7.96 1.7 7.76 1.62 7.6C1.56 7.43 1.51 7.26 1.47 7.08C1.43 6.9 1.3 6.76 1.12 6.72L0.62 6.63C0.39 6.59 0.23 6.4 0.23 6.17V5.17C0.23 4.94 0.39 4.75 0.62 4.71L1.12 4.62C1.3 4.59 1.43 4.45 1.47 4.27C1.51 4.1 1.56 3.93 1.62 3.77C1.7 3.59 1.7 3.4 1.6 3.24L1.34 2.81C1.22 2.62 1.27 2.36 1.44 2.23L2.28 1.54C2.44 1.41 2.67 1.41 2.83 1.53L3.26 1.82C3.41 1.92 3.6 1.94 3.77 1.88C3.94 1.81 4.11 1.75 4.29 1.7C4.46 1.66 4.6 1.52 4.63 1.33L4.71 0.83C4.75 0.6 4.94 0.44 5.17 0.44H6.24C6.47 0.44 6.66 0.6 6.7 0.83L6.78 1.33C6.81 1.52 6.95 1.66 7.12 1.7C7.3 1.75 7.47 1.81 7.64 1.88C7.81 1.94 7.99 1.92 8.15 1.82L8.58 1.53C8.74 1.41 8.97 1.41 9.13 1.54L9.97 2.23C10.14 2.36 10.18 2.62 10.07 2.81L9.81 3.24C9.71 3.4 9.71 3.59 9.79 3.77C9.85 3.93 9.91 4.1 9.95 4.27C9.99 4.45 10.12 4.59 10.3 4.62L10.8 4.71C11.03 4.75 11.19 4.94 11.19 5.17V6.17C11.19 6.4 11.03 6.59 10.8 6.63L10.3 6.72C10.12 6.76 9.99 6.9 9.95 7.08C9.91 7.26 9.85 7.43 9.79 7.6Z" stroke="currentColor" stroke-width="0.9"/>
          </svg>
          Settings
        </button>
      </div>
    </div>

    <AddProjectDialog
      :visible="showAddDialog"
      @close="showAddDialog = false"
      @created="handleProjectCreated"
    />
  </div>
</template>

