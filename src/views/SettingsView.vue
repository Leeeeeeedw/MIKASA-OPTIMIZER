<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { Bell, Database, Gauge, Globe, MonitorCog, RotateCcw, Save, ScanLine, Signal, Timer } from '@lucide/vue'
import { api } from '../api'
import { state, refreshDashboard, isScanActive } from '../store'
import type { AppSettings, SpeedSourceCompare, SpeedSourceProbe } from '../types'

const message = useMessage()
const busy = ref(false)
const form = reactive<AppSettings>({ ...state.settings })
const scanLocking = computed(() => isScanActive())

const probes = ref<SpeedSourceProbe[]>([])
const probing = ref(false)
const compareIp = ref('')
const compare = ref<SpeedSourceCompare | null>(null)
const comparing = ref(false)

async function runProbe() {
  probing.value = true
  try {
    probes.value = await api.speedSourceProbe()
  } catch (error) {
    message.error(String(error))
  } finally {
    probing.value = false
  }
}

async function runCompare() {
  const ip = compareIp.value.trim()
  if (!ip) {
    message.warning('请填写要对比的候选 IP')
    return
  }
  comparing.value = true
  try {
    compare.value = await api.speedSourceCompare(ip, form.defaultPort)
  } catch (error) {
    message.error(String(error))
  } finally {
    comparing.value = false
  }
}

function reset() {
  Object.assign(form, state.settings)
  message.info('已恢复为当前已保存设置')
}

async function save() {
  if (scanLocking.value) {
    message.warning('优选任务进行中，请等待完成或先取消任务后再保存设置')
    return
  }
  busy.value = true
  try {
    const saved = await api.saveSettings({ ...form })
    Object.assign(state.settings, saved)
    await refreshDashboard()
    message.success('设置已保存')
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function openData() {
  try {
    const path = await api.openDataDir()
    message.success(`数据目录：${path}`)
  } catch (error) {
    message.error(String(error))
  }
}
</script>

<template>
  <div class="page settings-page">
    <header class="page-header">
      <div>
        <h2>设置</h2>
        <p>普通设置与高级参数全部可修改，修改后立即应用于下一次扫描。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="reset"><RotateCcw :size="14" />恢复</button>
        <button class="btn-primary" :disabled="busy || scanLocking" @click="save"><Save :size="14" />保存设置</button>
      </div>
    </header>

    <div v-if="scanLocking" class="settings-lock">
      <ScanLine :size="14" />
      <span>当前扫描使用启动时的参数快照。为避免误操作，扫描进行期间设置保存已锁定。</span>
      <button class="btn-ghost" @click="state.activeView = 'scan'">查看任务进度</button>
    </div>

    <div class="settings-grid">
      <section class="glass-card settings-card">
        <div class="panel-head"><MonitorCog :size="16" /><span>基础设置</span></div>
        <div class="field-grid">
          <label>默认端口<input v-model.number="form.defaultPort" type="number" min="1" max="65535" /></label>
          <label>并发数量<input v-model.number="form.concurrency" type="number" min="1" max="512" /></label>
          <label>每地区保留数量<input v-model.number="form.perRegionTop" type="number" min="1" max="500" /></label>
          <label>TCP 延迟上限（ms）<input v-model.number="form.latencyLimitMs" type="number" min="1" /></label>
          <label>下载速度下限（MB/s）<input v-model.number="form.speedLimitMbS" type="number" step="0.1" min="0" /></label>
          <label>下载测试时长（秒）<input v-model.number="form.downloadSeconds" type="number" min="2" max="120" /></label>
          <label>最低有效数据量（MB）<input :value="Math.round(form.minDownloadBytes / 1024 / 1024)" type="number" min="1" @input="form.minDownloadBytes = Number(($event.target as HTMLInputElement).value) * 1024 * 1024" /></label>
          <label title="按历史真实成绩、地区配额和新节点探索机会综合选出，不按导入顺序截断">深度测试候选数<input v-model.number="form.maxCandidates" type="number" min="50" max="100000" /></label>
        </div>
        <div class="switch-row">
          <label class="switch-item"><input v-model="form.smartSupplement" type="checkbox" /><span><strong>智能补充模式</strong><em>结果不足时自动扩大候选池并复测</em></span></label>
          <label class="switch-item"><input v-model="form.autoDetectNetwork" type="checkbox" /><span><strong>自动检测网络</strong><em>仅显示省份和运营商，不显示 IP/坐标</em></span></label>
          <label class="switch-item"><input v-model="form.autoRefreshSources" type="checkbox" /><span><strong>自动更新订阅</strong><em>按间隔自动拉取网络链接来源</em></span></label>
        </div>
      </section>

      <section class="glass-card settings-card">
        <div class="panel-head"><Timer :size="16" /><span>测速参数</span></div>
        <div class="field-grid">
          <label>TCP 超时（ms）<input v-model.number="form.tcpTimeoutMs" type="number" min="100" /></label>
          <label>HTTPS 超时（ms）<input v-model.number="form.httpsTimeoutMs" type="number" min="100" /></label>
          <label>TCP 延迟测试轮数<input v-model.number="form.latencyRounds" type="number" min="1" max="10" /></label>
          <label title="同时下载测速的 IP 数量。1 = 单连接精准测速，最接近 v2rayN / CFData 的显示；数值越大扫描越快，但多个 IP 会分摊本机出口带宽">下载测速并发（1 = 精准）<input v-model.number="form.downloadThreads" type="number" min="1" max="32" /></label>
          <label>订阅刷新间隔（分钟）<input v-model.number="form.sourceRefreshMinutes" type="number" min="5" /></label>
          <label>日志等级
            <select v-model="form.logLevel">
              <option value="error">error</option><option value="warn">warn</option><option value="info">info</option><option value="debug">debug</option>
            </select>
          </label>
        </div>
        <label class="field-line">主测速地址<input v-model="form.downloadUrl" placeholder="https://speed.cloudflare.com/__down?bytes=104857600" /></label>
        <label class="field-line">备用测速地址<input v-model="form.backupDownloadUrl" placeholder="https://speed.cloudflare.com/__down?bytes=52428800" /></label>
        <label class="field-line">代理配置（可选）<input v-model="form.proxy" placeholder="http://127.0.0.1:7890" /></label>
        <p class="unit-note">速度单位：MB/s 表示每秒兆字节，1 MB/s ≈ 8 Mbps。CFData 界面同样写 MB/s，但源码按 MiB/s 计算；两者口径相差约 4.9%，不是速度偏慢的主要原因。</p>
      </section>

      <section class="glass-card settings-card">
        <div class="panel-head"><Signal :size="16" /><span>测速源</span></div>
        <div class="field-grid">
          <label>主测速源
            <select v-model="form.speedSource">
              <option value="auto">自动选择</option>
              <option value="cloudflare">Cloudflare 官方</option>
              <option value="cm">CM 提供</option>
              <option value="mobile">移动专属</option>
              <option value="custom">手动输入</option>
            </select>
          </label>
          <label>备用测速源
            <select v-model="form.backupSpeedSource">
              <option value="cloudflare">Cloudflare 官方</option>
              <option value="cm">CM 提供</option>
              <option value="mobile">移动专属</option>
              <option value="custom">手动输入</option>
            </select>
          </label>
        </div>
        <label class="field-line">手动测速地址<input v-model="form.customSpeedUrl" placeholder="https://example.com/__down?bytes=99999999" /></label>
        <div class="switch-row">
          <label class="switch-item"><input v-model="form.autoSwitchSpeedSource" type="checkbox" /><span><strong>首选源失败时自动切换备用源</strong><em>下载测速失败会记录原因，并尝试备用地址</em></span></label>
        </div>
        <button class="btn-ghost data-btn" :disabled="probing" @click="runProbe">
          <Signal :size="13" />{{ probing ? '预检中…' : '测速源预检' }}
        </button>
        <div v-if="probes.length" class="probe-list">
          <div v-for="item in probes" :key="item.key" class="probe-row">
            <span class="probe-name">{{ item.label }}</span>
            <span class="probe-state" :class="item.ok ? 'ok' : 'bad'">{{ item.ok ? `可用 ${item.status}` : '不可用' }}</span>
            <span class="probe-meta">{{ item.ok ? `${item.firstChunkBytes} B / ${item.elapsedMs} ms` : item.error }}</span>
          </div>
        </div>
        <div class="compare-box">
          <label class="field-line">测速源对比（同一 IP 依次用各源实测）
            <input v-model="compareIp" placeholder="例如 104.16.123.45" />
          </label>
          <button class="btn-ghost data-btn" :disabled="comparing" @click="runCompare">
            <Gauge :size="13" />{{ comparing ? '对比中…' : '开始对比' }}
          </button>
          <div v-if="compare" class="compare-result">
            <div class="compare-note">{{ compare.note }}</div>
            <div v-for="row in compare.rows" :key="row.key" class="probe-row">
              <span class="probe-name">{{ row.label }}</span>
              <span class="probe-state" :class="row.ok ? 'ok' : 'bad'">{{ row.ok ? `${row.mbS.toFixed(2)} MB/s` : '失败' }}</span>
              <span class="probe-meta">{{ row.ok ? `${row.connectedIp} · ${row.bytes} B / ${row.seconds.toFixed(1)}s` : row.error }}</span>
            </div>
          </div>
        </div>
      </section>

      <section class="glass-card settings-card">
        <div class="panel-head"><Globe :size="16" /><span>地区与 FOFA</span></div>
        <label class="field-line">FOFA 查询模板<textarea v-model="form.fofaQueryTemplate" rows="3" /></label>
        <label class="field-line">自定义 colo 映射<textarea v-model="form.customColoMap" rows="3" placeholder="可填写 JSON 或每行 CODE=地区key，用于补充 Cloudflare 新机房代码" /></label>
        <label class="field-line">FOFA 邮箱<input v-model="form.fofaEmail" placeholder="仅作为界面显示，API Key 在 IP 来源页面配置" /></label>
      </section>

      <section class="glass-card settings-card">
        <div class="panel-head"><Database :size="16" /><span>导出与 GitHub</span></div>
        <label class="field-line">默认导出目录<input v-model="form.exportDir" placeholder="留空使用文档目录" /></label>
        <label class="field-line">GitHub 默认分支<input v-model="form.githubDefaultBranch" placeholder="main" /></label>
        <label class="field-line">GitHub 远程目录<input v-model="form.githubRemoteDir" placeholder="mikasa-results" /></label>
        <div class="switch-row">
          <label class="switch-item"><input v-model="form.autoUploadAfterScan" type="checkbox" /><span><strong>扫描完成后自动上传</strong><em>需要先配置 GitHub Token</em></span></label>
        </div>
        <button class="btn-ghost data-btn" @click="openData"><Database :size="13" />打开数据目录</button>
      </section>
    </div>

    <div class="settings-footer">
      <span><Bell :size="12" />设置变化会实时保存到本地 SQLite，不会上传或写入日志。</span>
      <span><Gauge :size="12" />真实测速模式：所有阈值仅用于筛选，不会伪造速度。</span>
    </div>
  </div>
</template>

<style scoped>
.settings-page { gap: 14px; }

.settings-lock {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 13px;
  border: 1px solid rgba(242, 199, 135, 0.24);
  border-radius: 10px;
  background: rgba(242, 199, 135, 0.07);
  color: #e6b56b;
  font-size: 10px;
}

.settings-lock span { flex: 1; }
.unit-note { margin: 0; color: var(--text-3, #7a89a5); font-size: 9px; line-height: 1.6; }
.header-actions { display: flex; gap: 8px; }
.settings-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; min-width: 0; }
.settings-grid > * { min-width: 0; }
.settings-card { padding: 17px; display: flex; flex-direction: column; gap: 11px; }
.panel-head { display: flex; align-items: center; gap: 8px; color: #dbe5ff; font-size: 12px; font-weight: 700; }
.panel-head svg { color: #8ea9ff; }
.field-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 9px; }
.field-grid label, .field-line { display: flex; flex-direction: column; gap: 5px; color: #8290ae; font-size: 9.5px; }
.field-grid input, .field-grid select, .field-line input, .field-line textarea { min-height: 32px; border: 1px solid var(--line); border-radius: 6px; outline: none; background: rgba(5, 10, 23, 0.68); color: #c8d6f1; padding: 0 8px; font-size: 10px; }
.field-line textarea { min-height: 65px; padding: 8px; resize: vertical; font-family: var(--mono); line-height: 1.5; }
.switch-row { display: flex; flex-direction: column; gap: 8px; padding-top: 4px; }
.switch-item { display: flex; align-items: flex-start; gap: 8px; color: #8b9abb; font-size: 10px; }
.switch-item input { margin-top: 2px; accent-color: #6d8cff; }
.switch-item span { display: flex; flex-direction: column; gap: 3px; }
.switch-item strong { color: #acbcdc; font-weight: 500; }
.switch-item em { color: #5a6986; font-size: 8.5px; font-style: normal; }
.data-btn { width: fit-content; margin-top: 4px; }
.probe-list, .compare-result { display: flex; flex-direction: column; gap: 5px; padding: 8px; border: 1px solid var(--line); border-radius: 6px; background: rgba(5, 10, 23, 0.5); }
.probe-row { display: grid; grid-template-columns: 74px 96px 1fr; gap: 8px; align-items: center; font-size: 9.5px; color: #7f8dad; }
.probe-name { color: #b7c6e4; }
.probe-state { font-family: var(--mono); }
.probe-state.ok { color: #48cf9d; }
.probe-state.bad { color: #ef8a7c; }
.probe-meta { font-family: var(--mono); font-size: 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.compare-box { display: flex; flex-direction: column; gap: 7px; padding-top: 4px; border-top: 1px solid var(--line); }
.compare-note { color: #6d7c9b; font-size: 9px; }
.settings-footer { display: flex; justify-content: space-between; padding: 2px 2px 0; color: #586682; font-size: 9px; }
.settings-footer span { display: flex; align-items: center; gap: 5px; }
@media (max-width: 1310px) { .settings-grid { grid-template-columns: 1fr; } }

@media (max-width: 1180px) {
  .field-grid { grid-template-columns: 1fr; }
  .probe-row { grid-template-columns: 74px 96px minmax(0, 1fr); }
}
</style>
