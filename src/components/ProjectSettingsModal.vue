<script setup lang="ts">
import { ref, watch } from 'vue'
import { useProjectStore } from '../stores/projectStore'

const props = defineProps<{ visible: boolean; projectId: string }>()
const emit = defineEmits<{ close: [] }>()

const projectStore = useProjectStore()

const promptText = ref('')
const saving = ref(false)

watch(() => props.visible, (val) => {
  if (val) {
    const project = projectStore.projects.find(p => p.id === props.projectId)
    promptText.value = project?.system_prompt ?? ''
  }
})

async function save() {
  saving.value = true
  try {
    await projectStore.updateSystemPrompt(props.projectId, promptText.value || null)
    close()
  } finally {
    saving.value = false
  }
}

function close() {
  promptText.value = ''
  emit('close')
}
</script>

<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="modal ps-modal slide-up">
      <div class="modal-header">
        <h2>Project Settings</h2>
        <button class="btn-icon" @click="close" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="modal-body ps-body">
        <div class="ps-section">
          <div class="ps-section-label">Additional Instructions</div>
          <div class="ps-section-sub">Extra instructions appended to every task in this project. Claude Code already reads your project's CLAUDE.md and explores the repo on its own.</div>
          <textarea
            v-model="promptText"
            class="ps-prompt-textarea"
            placeholder="Add project-specific instructions..."
          />
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" @click="close">Cancel</button>
        <button class="btn btn-primary" :disabled="saving" @click="save">
          {{ saving ? 'Saving...' : 'Save' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ps-modal {
  width: 100%;
  max-width: 640px;
}

.ps-body {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.ps-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ps-section-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-secondary);
}

.ps-section-sub {
  font-size: 0.8125rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.ps-prompt-textarea {
  background: var(--bg-surface);
  border: 1px solid var(--border-hover);
  border-radius: var(--radius-xs);
  padding: 12px 14px;
  color: var(--text-primary);
  font-size: 0.8125rem;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  line-height: 1.65;
  outline: none;
  resize: vertical;
  min-height: 200px;
  width: 100%;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.ps-prompt-textarea:focus {
  border-color: var(--accent);
}

.ps-prompt-textarea::placeholder {
  color: var(--text-secondary);
}
</style>
