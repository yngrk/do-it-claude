<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useProjectStore } from '../stores/projectStore'
import ProjectCard from '../components/ProjectCard.vue'
import AddProjectDialog from '../components/AddProjectDialog.vue'

const projectStore = useProjectStore()
const showAddDialog = ref(false)

onMounted(() => {
  projectStore.loadProjects()
})
</script>

<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1>Projects</h1>
      <button class="btn btn-primary" @click="showAddDialog = true">+ Add Project</button>
    </div>

    <div v-if="projectStore.loading" class="loading">Loading projects...</div>
    <div v-else-if="projectStore.error" class="error-message">{{ projectStore.error }}</div>
    <div v-else-if="projectStore.projects.length === 0" class="empty-state">
      <p>No projects yet. Create one to get started.</p>
    </div>
    <div v-else class="project-grid">
      <ProjectCard
        v-for="project in projectStore.projects"
        :key="project.id"
        :project="project"
        @deleted="projectStore.loadProjects()"
      />
    </div>

    <AddProjectDialog
      :visible="showAddDialog"
      @close="showAddDialog = false"
    />
  </div>
</template>
