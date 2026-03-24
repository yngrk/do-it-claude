export interface Project {
  id: string
  name: string
  path: string
  created_at: string
}

export type TaskTag = string

export const DEFAULT_TASK_TAGS: { value: string; label: string; color: string }[] = [
  { value: 'bug',      label: 'Bug',      color: '#f87171' },
  { value: 'feature',  label: 'Feature',  color: '#60a5fa' },
  { value: 'docs',     label: 'Docs',     color: '#facc15' },
  { value: 'misc',     label: 'Misc',     color: '#94a3b8' },
]

export interface Task {
  id: string
  project_id: string
  title: string
  description: string
  tag: string | null
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

