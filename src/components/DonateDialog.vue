<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { Heart, LoaderCircle, LockKeyhole, X } from '@lucide/vue'
import { api } from '../api'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const tab = ref<'wechat' | 'alipay'>('wechat')
const qrUrls = ref<Record<'wechat' | 'alipay', string>>({ wechat: '', alipay: '' })
const qrLoading = ref(false)
const qrError = ref('')

async function loadQrCodes() {
  if (qrUrls.value.wechat && qrUrls.value.alipay) return
  qrLoading.value = true
  qrError.value = ''
  try {
    const [wechat, alipay] = await Promise.all([
      api.getDonationQr('wechat'),
      api.getDonationQr('alipay'),
    ])
    qrUrls.value = { wechat, alipay }
  } catch (error) {
    qrError.value = String(error)
  } finally {
    qrLoading.value = false
  }
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close')
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      tab.value = 'wechat'
      window.addEventListener('keydown', onKey)
      void loadQrCodes()
    } else {
      window.removeEventListener('keydown', onKey)
    }
  },
)

onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <Teleport to="body">
    <Transition name="donate-fade">
      <div v-if="open" class="donate-backdrop" @click.self="emit('close')">
        <section class="donate-dialog" role="dialog" aria-modal="true" aria-label="打赏作者">
          <button class="donate-close" title="关闭" @click="emit('close')"><X :size="16" /></button>

          <header class="donate-head">
            <span class="donate-head-icon"><Heart :size="17" /></span>
            <div>
              <h2>打赏作者</h2>
              <p>如果三笠优选帮到了你，欢迎请作者喝杯奶茶。</p>
            </div>
          </header>

          <div class="donate-tabs">
            <button :class="{ active: tab === 'wechat' }" @click="tab = 'wechat'">
              <i class="pay-dot wechat"></i>微信支付
            </button>
            <button :class="{ active: tab === 'alipay' }" @click="tab = 'alipay'">
              <i class="pay-dot alipay"></i>支付宝
            </button>
          </div>

          <div class="donate-body">
            <div class="donate-qr-card" :class="tab">
              <img
                v-if="qrUrls[tab]"
                :src="qrUrls[tab]"
                :alt="tab === 'wechat' ? '微信收款码' : '支付宝收款码'"
                draggable="false"
                @contextmenu.prevent
              />
              <div v-else class="qr-placeholder">
                <LoaderCircle v-if="qrLoading" :size="28" class="qr-spin" />
                <LockKeyhole v-else :size="28" />
                <strong>{{ qrLoading ? '正在安全加载收款码' : '收款码未随源码公开' }}</strong>
                <span>{{ qrError || '请使用作者发布的正式安装版查看' }}</span>
              </div>
            </div>
            <div class="donate-copy">
              <strong>{{ tab === 'wechat' ? '微信扫码打赏' : '支付宝扫码打赏' }}</strong>
              <p>{{ tab === 'wechat' ? '打开微信「扫一扫」，识别二维码即可。' : '打开支付宝「扫一扫」，支持花呗与信用卡。' }}</p>
              <ul>
                <li>打赏完全自愿，不影响任何功能</li>
                <li>不会解锁额外功能或改变测速结果</li>
                <li>你的支持会让这个工具继续更新</li>
              </ul>
              <span class="donate-note">感谢每一位认真测速、愿意支持的使用者。</span>
            </div>
          </div>

          <footer class="donate-foot">
            <span>MIKASA OPTIMIZER · 感谢支持</span>
            <button @click="emit('close')">关闭</button>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.donate-backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(3, 7, 18, 0.66);
  backdrop-filter: blur(7px);
}

.donate-dialog {
  position: relative;
  width: min(620px, calc(100vw - 48px));
  max-height: calc(100vh - 48px);
  overflow: auto;
  padding: 22px;
  border: 1px solid var(--line-strong);
  border-radius: 18px;
  background:
    radial-gradient(420px 220px at 88% 0%, rgba(180, 100, 255, 0.12), transparent 68%),
    var(--panel-solid);
  box-shadow: 0 30px 90px rgba(2, 6, 20, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.12);
}

.donate-close {
  position: absolute;
  right: 14px;
  top: 14px;
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
}

.donate-close:hover { color: var(--text-0); border-color: var(--line-strong); }

.donate-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding-right: 38px;
}

.donate-head-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  border: 1px solid rgba(255, 154, 205, 0.3);
  border-radius: 11px;
  background: linear-gradient(145deg, rgba(255, 154, 205, 0.2), rgba(150, 92, 255, 0.14));
  color: #ff9fca;
}

.donate-head h2 {
  margin: 0;
  color: var(--text-0);
  font-size: 18px;
  letter-spacing: 0.06em;
}

.donate-head p {
  margin: 5px 0 0;
  color: var(--text-2);
  font-size: 10px;
}

.donate-tabs {
  display: flex;
  gap: 7px;
  margin: 17px 0 15px;
}

.donate-tabs button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text-2);
  font-size: 10px;
  cursor: pointer;
}

.donate-tabs button.active {
  border-color: rgba(109, 140, 255, 0.45);
  background: rgba(109, 140, 255, 0.1);
  color: var(--text-0);
}

.pay-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.pay-dot.wechat { background: #22c55e; box-shadow: 0 0 7px rgba(34, 197, 94, 0.55); }
.pay-dot.alipay { background: #3b82f6; box-shadow: 0 0 7px rgba(59, 130, 246, 0.55); }

.donate-body {
  display: grid;
  grid-template-columns: 252px minmax(0, 1fr);
  gap: 20px;
  align-items: center;
}

.donate-qr-card {
  display: grid;
  place-items: center;
  padding: 10px;
  border-radius: 14px;
  background: #ffffff;
  box-shadow: 0 12px 32px rgba(4, 10, 28, 0.25);
}

.donate-qr-card.wechat { outline: 2px solid rgba(34, 197, 94, 0.24); }
.donate-qr-card.alipay { outline: 2px solid rgba(59, 130, 246, 0.24); }

.donate-qr-card img {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 7px;
  pointer-events: none;
  user-select: none;
  -webkit-user-drag: none;
}

.qr-placeholder {
  display: flex;
  min-height: 218px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 24px;
  color: #53627e;
  text-align: center;
}

.qr-placeholder strong {
  color: #23324d;
  font-size: 12px;
}

.qr-placeholder span {
  max-width: 190px;
  font-size: 9px;
  line-height: 1.65;
}

.qr-spin {
  animation: qr-spin 1s linear infinite;
}

@keyframes qr-spin {
  to { transform: rotate(360deg); }
}

.donate-copy strong {
  color: var(--text-0);
  font-size: 14px;
  letter-spacing: 0.03em;
}

.donate-copy p {
  margin: 8px 0 0;
  color: var(--text-2);
  font-size: 10px;
  line-height: 1.7;
}

.donate-copy ul {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin: 14px 0 0;
  padding: 0;
  list-style: none;
}

.donate-copy li {
  position: relative;
  padding-left: 13px;
  color: var(--text-2);
  font-size: 9.5px;
}

.donate-copy li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 5px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #ff9fca;
  box-shadow: 0 0 6px rgba(255, 159, 202, 0.5);
}

.donate-note {
  display: block;
  margin-top: 14px;
  padding-top: 11px;
  border-top: 1px solid var(--line);
  color: var(--text-3);
  font-size: 8.5px;
}

.donate-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 18px;
  padding-top: 13px;
  border-top: 1px solid var(--line);
  color: var(--text-3);
  font-size: 8.5px;
}

.donate-foot button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: transparent;
  color: var(--text-2);
  font-size: 9px;
  cursor: pointer;
}

.donate-foot button:hover { color: var(--text-0); border-color: var(--line-strong); }

.donate-fade-enter-active,
.donate-fade-leave-active { transition: opacity 0.18s ease; }
.donate-fade-enter-from,
.donate-fade-leave-to { opacity: 0; }

@media (max-width: 680px) {
  .donate-body { grid-template-columns: 1fr; }
  .donate-qr-card { width: min(252px, 100%); margin: 0 auto; }
}
</style>
