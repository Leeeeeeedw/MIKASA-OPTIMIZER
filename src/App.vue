<script setup lang="ts">
import { computed } from 'vue'
import {
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  dateZhCN,
  darkTheme,
  lightTheme,
  zhCN,
  type GlobalThemeOverrides,
} from 'naive-ui'
import AppShell from './components/AppShell.vue'
import { themeMode } from './theme'

const darkOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#6d8cff',
    primaryColorHover: '#8aa3ff',
    primaryColorPressed: '#526ee0',
    primaryColorSuppl: '#7c5cff',
    successColor: '#22c55e',
    warningColor: '#f59e0b',
    errorColor: '#f87171',
    bodyColor: 'transparent',
    cardColor: 'rgba(13, 20, 40, 0.72)',
    modalColor: '#0d1428',
    popoverColor: '#101a30',
    tableColor: 'transparent',
    inputColor: 'rgba(9, 15, 31, 0.72)',
    borderColor: 'rgba(112, 139, 255, 0.18)',
    dividerColor: 'rgba(112, 139, 255, 0.14)',
    textColorBase: '#e8edff',
    textColor1: '#e8edff',
    textColor2: '#aab4d8',
    textColor3: '#7b87ac',
    placeholderColor: '#596383',
    borderRadius: '10px',
    fontFamily: '"Microsoft YaHei UI", "Segoe UI", "PingFang SC", sans-serif',
  },
  Button: {
    borderRadiusMedium: '10px',
    fontWeight: '600',
  },
  Input: {
    borderRadius: '10px',
    color: 'rgba(7, 12, 26, 0.72)',
    colorFocus: 'rgba(9, 16, 34, 0.9)',
    border: '1px solid rgba(110, 135, 255, 0.18)',
    borderFocus: '1px solid rgba(109, 140, 255, 0.72)',
    boxShadowFocus: '0 0 0 3px rgba(109, 140, 255, 0.12)',
  },
  Select: {
    peers: {
      InternalSelection: {
        color: 'rgba(7, 12, 26, 0.72)',
        colorActive: 'rgba(9, 16, 34, 0.9)',
        border: '1px solid rgba(110, 135, 255, 0.18)',
        borderActive: '1px solid rgba(109, 140, 255, 0.72)',
        boxShadowActive: '0 0 0 3px rgba(109, 140, 255, 0.12)',
      },
    },
  },
  DataTable: {
    thColor: 'rgba(17, 27, 52, 0.8)',
    thTextColor: '#8794b8',
    tdColor: 'transparent',
    tdColorHover: 'rgba(109, 140, 255, 0.05)',
    borderColor: 'rgba(112, 139, 255, 0.1)',
  },
  Switch: {
    railColorActive: '#6d8cff',
  },
  Tabs: {
    tabTextColorActiveLine: '#dce4ff',
    tabTextColorHoverLine: '#dce4ff',
    barColor: '#7c5cff',
    tabTextColorLine: '#7b87ac',
  },
  Progress: {
    railColor: 'rgba(109, 140, 255, 0.12)',
    fillColor: 'linear-gradient(90deg, #6d8cff, #9b5cff)',
  },
}

const lightOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#4f6fe8',
    primaryColorHover: '#6683f1',
    primaryColorPressed: '#3b57c9',
    primaryColorSuppl: '#7455e6',
    successColor: '#16a34a',
    warningColor: '#d97706',
    errorColor: '#dc2626',
    bodyColor: 'transparent',
    cardColor: 'rgba(255, 255, 255, 0.9)',
    modalColor: '#ffffff',
    popoverColor: '#ffffff',
    tableColor: 'transparent',
    inputColor: '#ffffff',
    borderColor: 'rgba(58, 83, 140, 0.18)',
    dividerColor: 'rgba(58, 83, 140, 0.12)',
    textColorBase: '#17233d',
    textColor1: '#17233d',
    textColor2: '#465570',
    textColor3: '#6d7b96',
    placeholderColor: '#8a97af',
    borderRadius: '10px',
    fontFamily: '"Microsoft YaHei UI", "Segoe UI", "PingFang SC", sans-serif',
  },
  Button: { borderRadiusMedium: '10px', fontWeight: '600' },
  Input: {
    borderRadius: '10px',
    color: '#ffffff',
    colorFocus: '#ffffff',
    border: '1px solid rgba(58, 83, 140, 0.2)',
    borderFocus: '1px solid rgba(79, 111, 232, 0.72)',
    boxShadowFocus: '0 0 0 3px rgba(79, 111, 232, 0.1)',
  },
  Select: {
    peers: {
      InternalSelection: {
        color: '#ffffff',
        colorActive: '#ffffff',
        border: '1px solid rgba(58, 83, 140, 0.2)',
        borderActive: '1px solid rgba(79, 111, 232, 0.72)',
        boxShadowActive: '0 0 0 3px rgba(79, 111, 232, 0.1)',
      },
    },
  },
  DataTable: {
    thColor: '#eef3fb',
    thTextColor: '#53627e',
    tdColor: 'transparent',
    tdColorHover: 'rgba(79, 111, 232, 0.05)',
    borderColor: 'rgba(58, 83, 140, 0.1)',
  },
  Switch: { railColorActive: '#4f6fe8' },
  Tabs: {
    tabTextColorActiveLine: '#17233d',
    tabTextColorHoverLine: '#17233d',
    barColor: '#7455e6',
    tabTextColorLine: '#6d7b96',
  },
  Progress: {
    railColor: 'rgba(79, 111, 232, 0.14)',
    fillColor: 'linear-gradient(90deg, #4f6fe8, #7455e6)',
  },
}

const activeTheme = computed(() => (themeMode.value === 'light' ? lightTheme : darkTheme))
const activeOverrides = computed(() => (themeMode.value === 'light' ? lightOverrides : darkOverrides))
</script>

<template>
  <n-config-provider
    :theme="activeTheme"
    :theme-overrides="activeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <n-message-provider placement="top">
      <n-dialog-provider>
        <AppShell />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
