import { ref, watch } from 'vue'

export type ThemeMode = 'dark' | 'light'

// 每次启动固定使用夜间主题；主题按钮只影响当前会话。
export const themeMode = ref<ThemeMode>('dark')

function apply(mode: ThemeMode) {
  document.documentElement.dataset.theme = mode
  document.documentElement.style.colorScheme = mode
  localStorage.setItem('mikasa-theme', mode)
}

watch(themeMode, apply, { immediate: true })

export function toggleTheme() {
  themeMode.value = themeMode.value === 'dark' ? 'light' : 'dark'
}

export function setTheme(mode: ThemeMode) {
  themeMode.value = mode
}
