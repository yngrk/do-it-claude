<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import Sortable from 'sortablejs'
import TaskCard from './TaskCard.vue'
import type { Task } from '../types'

const props = defineProps<{
  title: string
  status: Task['status']
  tasks: Task[]
  allowDrag: boolean
}>()

const emit = defineEmits<{
  'task-moved': [taskId: string, newStatus: Task['status'], newSortOrder: number]
  'open-detail': [task: Task]
}>()

const listEl = ref<HTMLElement | null>(null)
let sortable: Sortable | null = null

onMounted(() => {
  if (!props.allowDrag || !listEl.value) return

  sortable = Sortable.create(listEl.value, {
    group: 'tasks',
    animation: 150,
    forceFallback: true,
    ghostClass: 'sortable-ghost',
    draggable: '.task-item',
    onAdd(evt) {
      const taskId = evt.item.dataset.taskId
      const newIndex = evt.newIndex ?? 0
      // Revert DOM: move node back to source so Vue's vdom stays in sync
      const origParent = evt.from
      const origIndex = evt.oldIndex ?? 0
      evt.item.parentNode?.removeChild(evt.item)
      if (origParent.children[origIndex]) {
        origParent.insertBefore(evt.item, origParent.children[origIndex])
      } else {
        origParent.appendChild(evt.item)
      }
      if (taskId) {
        emit('task-moved', taskId, props.status, newIndex)
      }
    },
    onUpdate(evt) {
      const taskId = evt.item.dataset.taskId
      const newIndex = evt.newIndex ?? 0
      if (taskId) {
        emit('task-moved', taskId, props.status, newIndex)
      }
    }
  })
})

onBeforeUnmount(() => {
  sortable?.destroy()
})
</script>

<template>
  <div :class="`kanban-column column-${status}`">
    <div class="column-header">
      <div class="column-header-left">
        <span class="column-title">{{ title }}</span>
        <span class="column-count">{{ tasks.length }}</span>
      </div>
      <slot name="actions"></slot>
    </div>

    <div ref="listEl" class="task-list" :data-status="status">
      <div
        v-for="task in tasks"
        :key="task.id"
        :data-task-id="task.id"
        class="task-item"
      >
        <TaskCard
          :task="task"
          @open-detail="emit('open-detail', task)"
        />
      </div>
      <div v-if="tasks.length === 0" class="empty-hint">
        {{ allowDrag ? 'Drop tasks here' : 'No tasks' }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.kanban-column {
  border-top: 2px solid transparent;
}

.column-backlog { border-top-color: #777; }
.column-queued { border-top-color: #3b82f6; }

.column-backlog .column-title { color: #999; }
.column-queued .column-title { color: #3b82f6; }

.empty-hint {
  text-align: center;
  padding: 24px 12px;
  color: var(--text-muted);
  font-size: 0.8rem;
  opacity: 0.5;
  user-select: none;
}

.task-list :deep(.sortable-ghost) {
  opacity: 0.3;
}
</style>
