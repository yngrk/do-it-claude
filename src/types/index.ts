export interface Project {
  id: string
  name: string
  path: string
  created_at: string
}

export interface Task {
  id: string
  project_id: string
  title: string
  description: string
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
