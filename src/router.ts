import { createRouter, createWebHashHistory } from 'vue-router'
import Dashboard from './views/Dashboard.vue'
import ProjectBoard from './views/ProjectBoard.vue'
import Settings from './views/Settings.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', component: Dashboard },
    { path: '/project/:id', component: ProjectBoard },
    { path: '/settings', component: Settings },
  ],
})

export default router
