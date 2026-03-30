export type CliProvider = 'claude' | 'codex'

export interface Project {
  id: string
  name: string
  path: string
  system_prompt: string | null
  mode_id: string | null
  created_at: string
}

export type TaskTag = string

export const DEFAULT_TASK_TAGS: { value: string; label: string; color: string }[] = [
  { value: 'bug',      label: 'Bug',      color: '#ef4444' },
  { value: 'feature',  label: 'Feature',  color: '#8b5cf6' },
  { value: 'update',   label: 'Update',   color: '#3b82f6' },
  { value: 'refactor', label: 'Refactor', color: '#f97316' },
  { value: 'docs',     label: 'Docs',     color: '#f59e0b' },
  { value: 'misc',     label: 'Misc',     color: '#6366f1' },
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
  max_turns: number | null
  model: string | null
  input_tokens: number | null
  output_tokens: number | null
  provider: string | null
  created_at: string
  started_at: string | null
  completed_at: string | null
  updated_at: string | null
}

export interface TokenEstimate {
  prompt_tokens: number
  system_tokens: number
  total_tokens: number
}

export interface TaskLog {
  id: string
  task_id: string
  content: string
  log_type: 'stdout' | 'stderr'
  created_at: string
}

export interface TaskMessage {
  id: string
  task_id: string
  role: 'user' | 'assistant'
  content: string
  message_type: 'chat'
  created_at: string
}

export interface ProjectMessage {
  id: string
  project_id: string
  role: 'user' | 'assistant'
  content: string
  created_at: string
}

export interface NotificationSettings {
  enabled: boolean
  notify_on_task_done: boolean
  notify_on_task_failed: boolean
  notify_on_chat_reply: boolean
  suppress_when_focused: boolean
}

export interface PromptTemplate {
  id: string
  name: string
  content: string
  created_at: string
  updated_at: string
}

export interface ProviderUsageStats {
  provider: string
  total_tasks: number
  completed_tasks: number
  failed_tasks: number
  total_input_tokens: number
  total_output_tokens: number
}

export interface ModeFile {
  id: string
  mode_id: string
  file_path: string
  content: string
}
