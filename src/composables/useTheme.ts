import { ref, watch } from 'vue'

export type Theme = 'dark' | 'light' | 'unhinged'

const theme = ref<Theme>((localStorage.getItem('theme') as Theme) || 'dark')

let unhingedInterval: ReturnType<typeof setInterval> | null = null
let clickListener: ((e: MouseEvent) => void) | null = null

function randomColor() {
  const h = Math.floor(Math.random() * 360)
  const s = 60 + Math.floor(Math.random() * 40)
  const l = 40 + Math.floor(Math.random() * 30)
  return `hsl(${h}, ${s}%, ${l}%)`
}

function randomLight() {
  const h = Math.floor(Math.random() * 360)
  const s = 70 + Math.floor(Math.random() * 30)
  const l = 70 + Math.floor(Math.random() * 20)
  return `hsl(${h}, ${s}%, ${l}%)`
}

function randomDark() {
  const h = Math.floor(Math.random() * 360)
  return `hsl(${h}, 30%, ${5 + Math.floor(Math.random() * 8)}%)`
}

function applyUnhinged() {
  const root = document.documentElement
  root.style.setProperty('--bg-base', randomDark())
  root.style.setProperty('--bg-surface', randomDark())
  root.style.setProperty('--bg-card', randomDark())
  root.style.setProperty('--bg-elevated', randomDark())
  root.style.setProperty('--bg-hover', randomDark())
  root.style.setProperty('--accent', randomLight())
  root.style.setProperty('--success', randomColor())
  root.style.setProperty('--warning', randomColor())
  root.style.setProperty('--error', randomColor())
  root.style.setProperty('--text-primary', randomLight())
  root.style.setProperty('--text-secondary', randomLight())
  root.style.setProperty('--text-muted', randomColor())
  root.style.setProperty('--border', `hsla(${Math.floor(Math.random() * 360)}, 80%, 50%, 0.3)`)
  root.style.setProperty('--border-hover', `hsla(${Math.floor(Math.random() * 360)}, 80%, 60%, 0.5)`)
  root.style.setProperty('--hover-overlay', `hsla(${Math.floor(Math.random() * 360)}, 80%, 50%, 0.15)`)
  root.style.setProperty('--badge-backlog-bg', randomColor())
  root.style.setProperty('--badge-backlog-color', randomLight())
  root.style.setProperty('--badge-queued-bg', randomColor())
  root.style.setProperty('--badge-queued-color', randomLight())
  root.style.setProperty('--badge-in_progress-bg', randomColor())
  root.style.setProperty('--badge-in_progress-color', randomLight())
  root.style.setProperty('--badge-done-bg', randomColor())
  root.style.setProperty('--badge-done-color', randomLight())
  root.style.setProperty('--badge-failed-bg', randomColor())
  root.style.setProperty('--badge-failed-color', randomLight())
  root.style.setProperty('--shadow-card', `0 4px 24px ${randomColor()}40`)
  root.style.setProperty('--shadow-card-hover', `0 8px 32px ${randomColor()}60`)
  root.style.setProperty('--bg-terminal', randomDark())
}

// --- Confetti ---
function spawnConfetti(x: number, y: number) {
  const count = 30 + Math.floor(Math.random() * 20)
  for (let i = 0; i < count; i++) {
    const el = document.createElement('div')
    const size = 4 + Math.random() * 8
    const angle = Math.random() * Math.PI * 2
    const velocity = 150 + Math.random() * 300
    const dx = Math.cos(angle) * velocity
    const dy = Math.sin(angle) * velocity - 100
    const rotation = Math.random() * 720 - 360
    const hue = Math.floor(Math.random() * 360)
    const shape = Math.random() > 0.5

    Object.assign(el.style, {
      position: 'fixed',
      left: `${x}px`,
      top: `${y}px`,
      width: `${shape ? size : size * 0.4}px`,
      height: `${shape ? size * 0.4 : size}px`,
      background: `hsl(${hue}, 90%, 60%)`,
      borderRadius: Math.random() > 0.5 ? '50%' : '1px',
      pointerEvents: 'none',
      zIndex: '99999',
    })

    document.body.appendChild(el)

    el.animate([
      { transform: 'translate(0, 0) rotate(0deg)', opacity: 1 },
      { transform: `translate(${dx}px, ${dy + 400}px) rotate(${rotation}deg)`, opacity: 0 },
    ], {
      duration: 800 + Math.random() * 600,
      easing: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)',
    }).onfinish = () => el.remove()
  }
}

// --- Flashbang ---
function flashbang() {
  const flash = document.createElement('div')
  Object.assign(flash.style, {
    position: 'fixed',
    inset: '0',
    background: `hsl(${Math.floor(Math.random() * 360)}, 100%, 85%)`,
    pointerEvents: 'none',
    zIndex: '99998',
  })
  document.body.appendChild(flash)

  flash.animate([
    { opacity: 0.9 },
    { opacity: 0 },
  ], {
    duration: 300,
    easing: 'ease-out',
  }).onfinish = () => flash.remove()
}

// --- Earthquake ---
function earthquake() {
  const app = document.getElementById('app')
  if (!app) return

  const frames: Keyframe[] = []
  for (let i = 0; i < 10; i++) {
    const x = (Math.random() - 0.5) * 16
    const y = (Math.random() - 0.5) * 12
    const r = (Math.random() - 0.5) * 3
    frames.push({ transform: `translate(${x}px, ${y}px) rotate(${r}deg)` })
  }
  frames.push({ transform: 'translate(0, 0) rotate(0deg)' })

  app.animate(frames, {
    duration: 500,
    easing: 'ease-out',
  })
}

// --- Click handler ---
function onUnhingedClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  const interactable = target.closest('button, a, [role="button"], .tab, .task-card, .task-item, .term-pill, .history-item, .project-card, .theme-option, .path-picker, input, textarea')
  if (!interactable) return

  spawnConfetti(e.clientX, e.clientY)
  flashbang()
  earthquake()
}

function clearUnhinged() {
  if (unhingedInterval) {
    clearInterval(unhingedInterval)
    unhingedInterval = null
  }
  if (clickListener) {
    document.removeEventListener('click', clickListener, true)
    clickListener = null
  }
  const root = document.documentElement
  root.removeAttribute('style')
}

function applyTheme(val: Theme) {
  clearUnhinged()
  document.documentElement.setAttribute('data-theme', val)
  localStorage.setItem('theme', val)

  if (val === 'unhinged') {
    applyUnhinged()
    unhingedInterval = setInterval(applyUnhinged, 2000)
    clickListener = onUnhingedClick
    document.addEventListener('click', clickListener, true)
  }
}

// Apply on load
applyTheme(theme.value)

watch(theme, applyTheme)

export function useTheme() {
  function setTheme(t: Theme) {
    theme.value = t
  }

  return { theme, setTheme }
}
