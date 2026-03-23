<script setup lang="ts">
import { ref, reactive, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

const props = defineProps<{ cwd: string }>()

interface TermSession {
  id: string
  name: string
  cwd: string
  terminal: Terminal
  fitAddon: FitAddon
  dead: boolean
  mounted: boolean
}

// All sessions across all projects
const allSessions = reactive<TermSession[]>([])
// Active session id per project: cwd -> sessionId
const activePerProject = reactive<Record<string, string>>({})

const termContainer = ref<HTMLElement | null>(null)

let unlistenOutput: (() => void) | null = null
let unlistenExit: (() => void) | null = null
let resizeObserver: ResizeObserver | null = null
let counterPerProject: Record<string, number> = {}

const termTheme = {
  background: '#050505',
  foreground: '#cccccc',
  cursor: '#cccccc',
  selectionBackground: '#333333',
  black: '#000000',
  red: '#ef4444',
  green: '#22c55e',
  yellow: '#eab308',
  blue: '#3b82f6',
  magenta: '#a855f7',
  cyan: '#06b6d4',
  white: '#cccccc',
  brightBlack: '#555555',
  brightRed: '#f87171',
  brightGreen: '#4ade80',
  brightYellow: '#facc15',
  brightBlue: '#60a5fa',
  brightMagenta: '#c084fc',
  brightCyan: '#22d3ee',
  brightWhite: '#ffffff',
}

// Sessions for the current project
function currentSessions() {
  return allSessions.filter(s => s.cwd === props.cwd)
}

function currentActiveId() {
  return activePerProject[props.cwd] ?? null
}

async function createSession() {
  const id = crypto.randomUUID()
  counterPerProject[props.cwd] = (counterPerProject[props.cwd] || 0) + 1

  const terminal = new Terminal({
    fontSize: 13,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    theme: termTheme,
    cursorBlink: true,
    scrollback: 5000,
    allowProposedApi: true,
  })

  const fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)

  const session: TermSession = {
    id,
    name: `zsh ${counterPerProject[props.cwd]}`,
    cwd: props.cwd,
    terminal,
    fitAddon,
    dead: false,
    mounted: false,
  }

  allSessions.push(session)
  activePerProject[props.cwd] = id

  await nextTick()
  mountSession(session)
}

function mountSession(session: TermSession) {
  if (!termContainer.value) return

  let wrapper = termContainer.value.querySelector(`[data-session="${session.id}"]`) as HTMLElement
  if (!wrapper) {
    wrapper = document.createElement('div')
    wrapper.setAttribute('data-session', session.id)
    wrapper.style.flex = '1'
    wrapper.style.height = '100%'
    termContainer.value.appendChild(wrapper)
    session.terminal.open(wrapper)
    session.mounted = true

    session.terminal.onData((data) => {
      invoke('write_pty', { sessionId: session.id, data })
    })

    invoke('open_pty', {
      sessionId: session.id,
      cwd: session.cwd,
      cols: session.terminal.cols,
      rows: session.terminal.rows,
    })
  }

  showCurrentProject()
}

function showCurrentProject() {
  if (!termContainer.value) return
  const activeId = currentActiveId()

  // Hide all, show only current project's active session
  for (const s of allSessions) {
    const el = termContainer.value.querySelector(`[data-session="${s.id}"]`) as HTMLElement
    if (el) {
      el.style.display = (s.cwd === props.cwd && s.id === activeId) ? '' : 'none'
    }
  }

  const session = allSessions.find(s => s.id === activeId)
  if (session) {
    nextTick(() => {
      session.fitAddon.fit()
      session.terminal.focus()
    })
  }
}

function switchTo(id: string) {
  activePerProject[props.cwd] = id
  showCurrentProject()
}

async function closeSession(id: string, e: MouseEvent) {
  e.stopPropagation()
  const idx = allSessions.findIndex(s => s.id === id)
  if (idx === -1) return

  const session = allSessions[idx]
  const wasCwd = session.cwd
  try { await invoke('close_pty', { sessionId: id }) } catch {}
  session.terminal.dispose()

  if (termContainer.value) {
    termContainer.value.querySelector(`[data-session="${id}"]`)?.remove()
  }

  allSessions.splice(idx, 1)

  if (activePerProject[wasCwd] === id) {
    const remaining = allSessions.filter(s => s.cwd === wasCwd)
    activePerProject[wasCwd] = remaining.length > 0 ? remaining[0].id : ''
    if (wasCwd === props.cwd) showCurrentProject()
  }
}

// Watch for project changes
watch(() => props.cwd, () => {
  nextTick(() => {
    const sessions = currentSessions()
    if (sessions.length === 0) {
      createSession()
    } else {
      showCurrentProject()
    }
  })
})

onMounted(async () => {
  unlistenOutput = await listen<[string, string]>('pty-output', (event) => {
    const [sid, data] = event.payload
    const session = allSessions.find(s => s.id === sid)
    if (session) session.terminal.write(data)
  })

  unlistenExit = await listen<string>('pty-exit', (event) => {
    const session = allSessions.find(s => s.id === event.payload)
    if (session) {
      session.dead = true
      session.terminal.write('\r\n\x1b[90m[Process exited]\x1b[0m\r\n')
    }
  })

  if (termContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      const activeId = currentActiveId()
      const session = allSessions.find(s => s.id === activeId)
      if (session) {
        session.fitAddon.fit()
        invoke('resize_pty', {
          sessionId: session.id,
          cols: session.terminal.cols,
          rows: session.terminal.rows,
        })
      }
    })
    resizeObserver.observe(termContainer.value)
  }

  createSession()
})

onBeforeUnmount(() => {
  // Only clean up listeners and observer — keep PTY sessions alive
  unlistenOutput?.()
  unlistenExit?.()
  resizeObserver?.disconnect()
  // Dispose xterm instances (DOM is going away)
  for (const s of allSessions) {
    s.terminal.dispose()
    s.mounted = false
  }
})
</script>

<template>
  <div class="term-panel">
    <div class="term-header">
      <div class="term-pills">
        <button
          v-for="s in currentSessions()"
          :key="s.id"
          class="term-pill"
          :class="{ 'term-pill-active': currentActiveId() === s.id, 'term-pill-dead': s.dead }"
          @click="switchTo(s.id)"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M1.5 2.5L4 5L1.5 7.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M5 7.5H8.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
          {{ s.name }}
          <span class="pill-close" @click="closeSession(s.id, $event)">
            <svg width="7" height="7" viewBox="0 0 7 7" fill="none">
              <path d="M1 1L6 6M6 1L1 6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
            </svg>
          </span>
        </button>
      </div>
      <button class="term-new" @click="createSession" title="New terminal">
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
          <path d="M5 1V9M1 5H9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <div ref="termContainer" class="term-container"></div>
  </div>
</template>

<style scoped>
.term-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.term-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.term-pills {
  display: flex;
  align-items: center;
  gap: 3px;
  overflow-x: auto;
  flex: 1;
  min-width: 0;
}

.term-pills::-webkit-scrollbar { height: 0; }

.term-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 0.6875rem;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
  white-space: nowrap;
  flex-shrink: 0;
}

.term-pill:hover {
  color: var(--text-secondary);
  background: var(--hover-overlay);
}

.term-pill-active {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.term-pill-dead {
  opacity: 0.4;
}

.pill-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  border-radius: 3px;
  opacity: 0;
  transition: opacity 0.1s ease;
  color: var(--text-muted);
}

.term-pill:hover .pill-close {
  opacity: 0.5;
}

.pill-close:hover {
  opacity: 1 !important;
  background: var(--hover-overlay);
  color: var(--text-primary);
}

.term-new {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s ease, background 0.15s ease;
}

.term-new:hover {
  color: var(--text-primary);
  background: var(--hover-overlay);
}

.term-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #050505;
  position: relative;
  overflow: hidden;
}

.term-container :deep(.xterm) {
  height: 100%;
  padding: 8px 0 0 12px;
}

.term-container :deep(.xterm-viewport) {
  overflow-y: auto !important;
}

.term-container :deep(.xterm-screen) {
  padding-bottom: 8px;
}
</style>
