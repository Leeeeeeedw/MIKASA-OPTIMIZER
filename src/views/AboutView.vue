<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  CircleCheck,
  GitBranch,
  Heart,
  Info,
  ShieldCheck,
  Sparkles,
  Zap,
} from '@lucide/vue'
import { api } from '../api'
import { state } from '../store'
import mikasaChild from '../assets/young-mikasa-rail.png'
import sakuraBranch from '../assets/sakura-branch.svg'
import wingsOfFreedom from '../assets/wings-of-freedom.svg'

const info = ref<{ version: string; dataDir: string; dbPath: string }>({
  version: state.version,
  dataDir: '',
  dbPath: '',
})

onMounted(async () => {
  try {
    info.value = await api.appInfo()
  } catch {
    /* 读取失败时保留商店中的版本号，页面其余内容不受影响 */
  }
})
</script>

<template>
  <div class="page about-page">
    <section class="about-stage">
      <div class="stage-sky"></div>
      <div class="stage-aurora"></div>
      <div class="stage-stars"></div>
      <img class="stage-branch" :src="sakuraBranch" alt="" aria-hidden="true" />
      <div class="stage-petals" aria-hidden="true">
        <i v-for="n in 9" :key="n"></i>
      </div>
      <div class="stage-halo"></div>
      <div class="stage-portrait-wrap">
        <img class="stage-portrait" :src="mikasaChild" alt="童年三笠立绘" />
      </div>

      <div class="stage-copy">
        <div class="stage-eyebrow-row">
          <img class="stage-emblem" :src="wingsOfFreedom" alt="" aria-hidden="true" />
          <span class="stage-kicker">Cloudflare Edge Optimizer</span>
        </div>
        <h1>三笠优选</h1>
        <div class="stage-latin">MIKASA <b>OPTIMIZER</b></div>
        <p class="stage-tagline">更优的连接，更好的体验</p>
        <p class="stage-desc">七地区并发优选 · 真实下载测速 · 城市级节点识别</p>
        <div class="stage-chips">
          <span class="stage-version">v{{ info.version }}</span>
          <span><ShieldCheck :size="11" />本地运行</span>
          <span><Zap :size="11" />七地区覆盖</span>
          <span><Sparkles :size="11" />真实测速</span>
        </div>
      </div>

      <div class="stage-foot">© 2026 三笠优选 · 让网络，更进一步</div>
    </section>

    <section class="about-grid">
      <article class="glass-card about-card">
        <div class="panel-head"><Sparkles :size="16" /><span>软件定位</span></div>
        <p>
          面向普通用户的 Cloudflare IP 优选桌面工具。多来源统一候选池，七地区独立扫描，
          真实 TCP、HTTPS、延迟与下载速度测试，输出可直接导入客户端的节点列表。
        </p>
        <div class="feature-tags">
          <span>真实下载测速</span>
          <span>CF-RAY colo 识别</span>
          <span>城市级显示</span>
          <span>七地区独立</span>
          <span>SQLite 历史</span>
          <span>GitHub 上传</span>
        </div>
      </article>

      <article class="glass-card about-card">
        <div class="panel-head"><ShieldCheck :size="16" /><span>数据与隐私</span></div>
        <p>
          FOFA API Key、GitHub Token 保存在 Windows 凭据管理器，不写入配置文件或日志。
          本地网络状态只显示省份与运营商，不显示公网 IP、街道、坐标或城市级详细位置。
        </p>
        <div class="data-path"><span>数据目录</span><code>{{ info.dataDir || '—' }}</code></div>
        <div class="data-path"><span>数据库</span><code>{{ info.dbPath || '—' }}</code></div>
      </article>

      <article class="glass-card about-card">
        <div class="panel-head"><Zap :size="16" /><span>测速原则</span></div>
        <ul>
          <li><CircleCheck :size="12" />每个正式结果都有真实延迟与真实下载速度</li>
          <li><CircleCheck :size="12" />下载失败、速度不达标不得进入正式名单</li>
          <li><CircleCheck :size="12" />不按延迟单独决定排名，支持综合质量排序</li>
          <li><CircleCheck :size="12" />地区以实际 HTTPS 请求 colo 为准</li>
        </ul>
      </article>

      <article class="glass-card about-card">
        <div class="panel-head"><GitBranch :size="16" /><span>开源与致谢</span></div>
        <p>
          测速思路参考 XIU2/CloudflareSpeedTest 与 CFData-WEB；Cloudflare 官方 IP 段来自
          cloudflare.com/ips。第三方组件遵循各自许可证，版权归原作者所有。
        </p>
        <div class="thanks"><Heart :size="12" />感谢每一位真实测速、认真筛选的使用者。</div>
      </article>
    </section>

    <footer class="about-footer">
      <span><Info :size="12" />MIKASA OPTIMIZER · 本地运行，不上传测速结果</span>
      <span>Windows 10 / 11 · x64</span>
    </footer>
  </div>
</template>

<style scoped>
.about-page { gap: 14px; }

.about-stage {
  position: relative;
  min-height: 286px;
  height: clamp(286px, 40vh, 372px);
  padding: 30px 34px;
  overflow: hidden;
  isolation: isolate;
  border: 1px solid rgba(116, 140, 255, 0.22);
  border-radius: 18px;
  background: #070b1c;
  box-shadow:
    0 24px 70px rgba(2, 7, 24, 0.55),
    inset 0 1px 0 rgba(180, 199, 255, 0.14);
}

.stage-sky,
.stage-aurora,
.stage-stars,
.stage-portrait-wrap,
.stage-copy,
.stage-foot,
.stage-branch,
.stage-petals,
.stage-halo {
  position: absolute;
}

.stage-sky {
  inset: 0;
  background:
    radial-gradient(85% 150% at 8% 10%, rgba(96, 72, 190, 0.45), transparent 58%),
    radial-gradient(70% 130% at 62% 18%, rgba(52, 80, 190, 0.35), transparent 62%),
    radial-gradient(90% 120% at 100% 100%, rgba(124, 70, 190, 0.42), transparent 62%),
    linear-gradient(118deg, #0a0f26 0%, #0c1433 42%, #131038 72%, #090b1f 100%);
}

.stage-aurora {
  left: 24%;
  top: -46%;
  width: 56%;
  height: 110%;
  background: conic-gradient(
    from 210deg at 50% 50%,
    transparent 0deg,
    rgba(116, 152, 255, 0.16) 42deg,
    rgba(166, 109, 255, 0.2) 96deg,
    transparent 150deg,
    rgba(76, 210, 255, 0.08) 210deg,
    transparent 300deg
  );
  filter: blur(26px);
  opacity: 0.85;
  transform: rotate(-8deg);
}

.stage-stars {
  inset: 0;
  background-image:
    radial-gradient(1.4px 1.4px at 9% 22%, rgba(255, 255, 255, 0.95), transparent 60%),
    radial-gradient(1.2px 1.2px at 18% 68%, rgba(198, 219, 255, 0.85), transparent 60%),
    radial-gradient(1.6px 1.6px at 27% 34%, rgba(255, 255, 255, 0.8), transparent 60%),
    radial-gradient(1px 1px at 36% 12%, rgba(224, 233, 255, 0.9), transparent 60%),
    radial-gradient(1.3px 1.3px at 44% 57%, rgba(205, 220, 255, 0.72), transparent 60%),
    radial-gradient(1.5px 1.5px at 53% 26%, rgba(255, 255, 255, 0.85), transparent 60%),
    radial-gradient(1px 1px at 61% 76%, rgba(214, 227, 255, 0.72), transparent 60%),
    radial-gradient(1.2px 1.2px at 72% 18%, rgba(255, 255, 255, 0.78), transparent 60%),
    radial-gradient(1.5px 1.5px at 82% 63%, rgba(233, 221, 255, 0.65), transparent 60%),
    radial-gradient(1px 1px at 92% 32%, rgba(255, 255, 255, 0.72), transparent 60%);
  opacity: 0.8;
}

.stage-branch {
  left: -168px;
  top: -44px;
  width: 380px;
  height: auto;
  opacity: 0.42;
  transform: scaleX(-1) rotate(-6deg);
  filter: brightness(1.15) saturate(1.2) drop-shadow(0 0 18px rgba(255, 154, 205, 0.26));
  pointer-events: none;
}

.stage-halo {
  right: 2%;
  top: 2%;
  width: 430px;
  height: 430px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(139, 96, 255, 0.38), rgba(74, 83, 198, 0.12) 48%, transparent 70%);
  filter: blur(14px);
}

.stage-portrait-wrap {
  right: 0;
  top: 0;
  width: min(46%, 440px);
  height: 100%;
  overflow: hidden;
  mask-image:
    linear-gradient(90deg, transparent 0%, #000 20%, #000 100%),
    linear-gradient(0deg, transparent 0%, #000 16%, #000 100%);
  mask-composite: intersect;
  -webkit-mask-image:
    linear-gradient(90deg, transparent 0%, #000 20%, #000 100%),
    linear-gradient(0deg, transparent 0%, #000 16%, #000 100%);
  -webkit-mask-composite: source-in;
}

.stage-portrait {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: 50% 16%;
  filter: saturate(1.08) brightness(0.98) contrast(1.02);
  transform: scale(1.02);
}

.stage-copy {
  left: 34px;
  top: 50%;
  z-index: 3;
  width: min(56%, 560px);
  transform: translateY(-52%);
}

.stage-kicker {
  display: inline-flex;
  align-items: center;
  padding: 4px 9px;
  border: 1px solid rgba(145, 166, 255, 0.26);
  border-radius: 99px;
  background: rgba(96, 119, 226, 0.12);
  color: #a9b9f2;
  font-size: 8px;
  font-weight: 700;
  letter-spacing: 0.22em;
  text-transform: uppercase;
}

.stage-eyebrow-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.stage-emblem {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  box-shadow: 0 0 20px rgba(109, 140, 255, 0.45);
}

.stage-copy h1 {
  margin: 12px 0 0;
  color: #f5f8ff;
  font-size: clamp(38px, 4.2vw, 54px);
  line-height: 1;
  letter-spacing: 0.12em;
  font-weight: 800;
  text-shadow: 0 0 34px rgba(139, 161, 255, 0.4);
}

.stage-latin {
  margin-top: 9px;
  color: #c9b7ff;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.46em;
}

.stage-latin b {
  color: #8fa6ea;
  font-weight: 600;
}

.stage-tagline {
  margin: 18px 0 0;
  color: #dfe8ff;
  font-size: 15px;
  font-weight: 500;
  letter-spacing: 0.15em;
}

.stage-tagline::after {
  content: "";
  display: block;
  width: 176px;
  height: 1px;
  margin-top: 10px;
  background: linear-gradient(90deg, rgba(154, 172, 255, 0.85), rgba(173, 112, 255, 0.35), transparent);
}

.stage-desc {
  margin: 11px 0 0;
  color: #8999bd;
  font-size: 10px;
  letter-spacing: 0.05em;
}

.stage-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 17px;
}

.stage-chips > span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 9px;
  border: 1px solid rgba(133, 154, 235, 0.2);
  border-radius: 99px;
  background: rgba(15, 23, 52, 0.56);
  color: #93a3c8;
  font-size: 8.5px;
  backdrop-filter: blur(6px);
}

.stage-chips svg { color: #8fa3e0; }

.stage-chips .stage-version {
  border-color: rgba(158, 121, 255, 0.35);
  background: linear-gradient(120deg, rgba(89, 104, 234, 0.32), rgba(146, 88, 226, 0.24));
  color: #cdd9ff;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.stage-foot {
  right: 20px;
  bottom: 14px;
  z-index: 3;
  color: rgba(163, 177, 210, 0.62);
  font-size: 8.5px;
  letter-spacing: 0.06em;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.55);
}

.stage-petals { inset: 0; z-index: 2; pointer-events: none; }

.stage-petals i {
  position: absolute;
  top: -12px;
  width: 7px;
  height: 5px;
  border-radius: 60% 40% 60% 40%;
  background: linear-gradient(120deg, #ffd8e9, #e99bc4);
  opacity: 0;
  box-shadow: 0 0 7px rgba(255, 176, 214, 0.45);
  animation: petal-drift 15s linear infinite;
}

.stage-petals i:nth-child(1) { left: 8%; animation-delay: 0s; }
.stage-petals i:nth-child(2) { left: 19%; animation-delay: 3.4s; animation-duration: 18s; transform: scale(0.75); }
.stage-petals i:nth-child(3) { left: 31%; animation-delay: 6.8s; animation-duration: 16s; }
.stage-petals i:nth-child(4) { left: 42%; animation-delay: 1.8s; animation-duration: 20s; transform: scale(0.62); }
.stage-petals i:nth-child(5) { left: 55%; animation-delay: 8.2s; animation-duration: 17s; }
.stage-petals i:nth-child(6) { left: 64%; animation-delay: 4.6s; animation-duration: 19s; transform: scale(0.8); }
.stage-petals i:nth-child(7) { left: 74%; animation-delay: 10.4s; animation-duration: 15s; transform: scale(0.6); }
.stage-petals i:nth-child(8) { left: 84%; animation-delay: 7.2s; animation-duration: 21s; }
.stage-petals i:nth-child(9) { left: 93%; animation-delay: 2.6s; animation-duration: 18s; transform: scale(0.72); }

@keyframes petal-drift {
  0% { opacity: 0; translate: 0 -10px; rotate: 0deg; }
  12% { opacity: 0.75; }
  70% { opacity: 0.55; }
  100% { opacity: 0; translate: -46px 320px; rotate: 210deg; }
}

.about-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.about-card {
  padding: 17px 18px;
}

.panel-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 9px;
  color: var(--text-0);
  font-size: 12px;
  font-weight: 700;
}

.panel-head svg { color: var(--blue-bright); }

.about-card p {
  margin: 0;
  color: var(--text-2);
  font-size: 10px;
  line-height: 1.85;
}

.feature-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 11px;
}

.feature-tags span {
  padding: 4px 8px;
  border: 1px solid var(--line);
  border-radius: 5px;
  color: var(--text-2);
  font-size: 8.5px;
}

.data-path {
  display: flex;
  gap: 8px;
  margin-top: 8px;
  color: var(--text-3);
  font-size: 8.5px;
}

.data-path span { flex-shrink: 0; }

.data-path code {
  color: var(--text-1);
  font-family: var(--mono);
  word-break: break-all;
}

.about-card ul {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.about-card li {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--text-2);
  font-size: 9.5px;
  line-height: 1.55;
}

.about-card li svg {
  flex-shrink: 0;
  margin-top: 1px;
  color: #47ce8c;
}

.thanks {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 11px;
  color: #c48cb9;
  font-size: 9px;
}

.about-footer {
  display: flex;
  justify-content: space-between;
  padding: 0 2px 2px;
  color: var(--text-3);
  font-size: 9px;
}

.about-footer span {
  display: flex;
  align-items: center;
  gap: 5px;
}

@media (max-width: 1180px) {
  .about-grid { grid-template-columns: 1fr; }
  .stage-copy { width: 62%; }
  .stage-portrait-wrap { width: 50%; opacity: 0.85; }
  .stage-tagline { font-size: 13px; }
}

@media (max-width: 860px) {
  .about-stage { height: auto; min-height: 330px; padding: 26px; }
  .stage-copy { position: relative; left: 0; top: 0; width: 100%; transform: none; }
  .stage-portrait-wrap { width: 72%; opacity: 0.34; }
  .stage-foot { display: none; }
}

@media (prefers-reduced-motion: reduce) {
  .stage-petals { display: none; }
}
</style>
