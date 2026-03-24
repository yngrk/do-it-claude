export interface Project {
  id: string
  name: string
  path: string
  created_at: string
}

export type TaskTag = 'bug' | 'feature' | 'refactor' | 'test' | 'docs' | 'chore'

export const TASK_TAGS: { value: TaskTag; label: string; color: string }[] = [
  { value: 'bug',      label: 'Bug',      color: '#f87171' },
  { value: 'feature',  label: 'Feature',  color: '#60a5fa' },
  { value: 'refactor', label: 'Refactor', color: '#c084fc' },
  { value: 'test',     label: 'Test',     color: '#4ade80' },
  { value: 'docs',     label: 'Docs',     color: '#facc15' },
  { value: 'chore',    label: 'Chore',    color: '#94a3b8' },
]

export interface Task {
  id: string
  project_id: string
  title: string
  description: string
  tag: TaskTag | null
  status: 'backlog' | 'queued' | 'in_progress' | 'done' | 'failed'
  sort_order: number
  exit_code: number | null
  created_at: string
  started_at: string | null
  completed_at: string | null
}

export interface TaskLog {
  id: string
  task_id: string
  content: string
  log_type: 'stdout' | 'stderr'
  created_at: string
}
