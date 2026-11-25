import { createCipheriv, randomBytes } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const keyFile = resolve(root, '.env.donation.local')
const outputDir = resolve(root, 'src-tauri', 'resources', 'donation')
const assets = [
  ['wechat', resolve(root, 'private', 'donation', 'donate-wechat.png')],
  ['alipay', resolve(root, 'private', 'donation', 'donate-alipay.png')],
]

function readKey() {
  if (process.env.MIKASA_DONATE_KEY) {
    return Buffer.from(process.env.MIKASA_DONATE_KEY, 'base64')
  }
  if (existsSync(keyFile)) {
    const match = readFileSync(keyFile, 'utf8').match(/^MIKASA_DONATE_KEY=(.+)$/m)
    if (match) return Buffer.from(match[1].trim(), 'base64')
  }
  return randomBytes(32)
}

const key = readKey()
if (key.length !== 32) {
  throw new Error('MIKASA_DONATE_KEY 必须是 32 字节密钥的 Base64 编码')
}

for (const [name, path] of assets) {
  if (!existsSync(path)) throw new Error(`缺少打赏码原图: ${path}`)
  const plain = readFileSync(path)
  const nonce = randomBytes(12)
  const cipher = createCipheriv('aes-256-gcm', key, nonce)
  cipher.setAAD(Buffer.from(`mikasa-donation:${name}`, 'utf8'))
  const encrypted = Buffer.concat([cipher.update(plain), cipher.final()])
  const payload = Buffer.concat([Buffer.from('MKSQ1', 'ascii'), nonce, encrypted, cipher.getAuthTag()])
  writeFileSync(resolve(outputDir, `${name}.bin`), payload)
  console.log(`encrypted ${name}: ${plain.length} -> ${payload.length} bytes`)
}

if (!existsSync(keyFile)) {
  mkdirSync(dirname(keyFile), { recursive: true })
  writeFileSync(keyFile, `MIKASA_DONATE_KEY=${key.toString('base64')}\n`, { mode: 0o600 })
  console.log(`key written to ${keyFile}`)
}
