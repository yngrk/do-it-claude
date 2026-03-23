<script setup lang="ts">
import { useProjectStore } from '../stores/projectStore'

const projectStore = useProjectStore()
</script>

<template>
  <div class="welcome fade-in">
    <div class="welcome-content">
      <svg width="48" height="48" viewBox="0 0 20 20" fill="none" class="welcome-icon">
        <path d="M10 2L13 7H17L14 11L15.5 16L10 13L4.5 16L6 11L3 7H7L10 2Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/>
      </svg>
      <h1 class="gradient-text">Do It Claude</h1>
      <p class="welcome-tagline">Batch your Claude tasks. Walk away.</p>
      <p v-if="projectStore.projects.length === 0" class="welcome-hint">
        Hit the <strong>+</strong> button to create your first project.
      </p>
      <p v-else class="welcome-hint">
        Select a project tab above to get started.
      </p>
    </div>
  </div>
</template>

<style scoped>
.welcome {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: calc(100vh - 120px);
}

.welcome-content {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.welcome-icon {
  margin-bottom: 8px;
  opacity: 0.8;
}

.welcome-content h1 {
  font-size: 1.75rem;
  letter-spacing: -0.04em;
}

.welcome-tagline {
  color: var(--text-secondary);
  font-size: 0.9375rem;
}

.welcome-hint {
  color: var(--text-muted);
  font-size: 0.8125rem;
  margin-top: 8px;
}

.welcome-hint strong {
  color: var(--text-secondary);
}
</style>
