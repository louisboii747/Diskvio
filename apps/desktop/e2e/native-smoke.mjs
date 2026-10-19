import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { mkdir } from 'node:fs/promises'

// Attach only to a deliberately launched local test WebView. No browser mocks.
const browser = await chromium.connectOverCDP('http://127.0.0.1:9223')
try {
  const pages = browser.contexts().flatMap((context) => context.pages())
  assert.equal(pages.length, 1, 'Expected exactly one Diskvio test webview')
  const page = pages[0]
  if (process.argv.includes('--packaged')) {
    await page.waitForURL(/^https?:\/\/tauri\.localhost/)
  } else {
    await page.goto('http://127.0.0.1:1420')
  }
  await page.getByRole('heading', { level: 1, name: 'Disk Overview' }).waitFor()
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByText('Connected through Tauri IPC').waitFor()
  assert.match(await page.locator('.about-details').innerText(), /windows/)
  assert.match(await page.locator('.about-details').innerText(), /All unavailable/)
  assert.equal(
    await page.getByRole('alert').count(),
    0,
    'Native menu event listener must register successfully',
  )
  await mkdir('test-results', { recursive: true })
  await page.screenshot({ path: 'test-results/native-ipc.png', fullPage: true })
  console.log(
    'PASS: actual Windows WebView rendered; get_app_info IPC connected; capabilities denied; native event subscription succeeded.',
  )
} finally {
  await browser.close()
}
