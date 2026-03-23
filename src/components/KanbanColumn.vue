<script setup lang="ts">
import draggable from 'vuedraggable'
import TaskCard from './TaskCard.vue'
import type { Task } from '../types'

const props = defineProps<{
  title: string
  status: Task['status']
  tasks: Task[]
}>()

const emit = defineEmits<{
  'task-moved': [taskId: string, newStatus: Task['status'], newSortOrder: number]
  'open-detail': [task: Task]
}>()

function onDragEnd(event: { added?: { element: Task; newIndex: number } }) {
  if (event.added) {
    const { element, newIndex } = event.added
    emit('task-moved', element.id, props.status, newIndex)
  }
}
</script>

<template>
  <div class="kanban-column">
    <div class="column-header">
      <span class="column-title">{{ title }}</span>
      <span class="column-count">{{ tasks.length }}</span>
    </div>
    <draggable
      :list="tasks"
      item-key="id"
      group="tasks"
      class="task-list"
      @change="onDragEnd"
    >
      <template #item="{ element }">
        <TaskCard
          :task="element"
          @open-detail="emit('open-detail', element)"
        />
      </template>
    </draggable>
  </div>
</template>
