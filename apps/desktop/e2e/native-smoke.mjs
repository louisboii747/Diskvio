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
  await page.getByRole('heading', { level: 1, name: 'Drives' }).waitFor()
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByText('Connected', { exact: true }).waitFor()
  assert.match(await page.locator('.about-details').innerText(), /Windows/)
  assert.match(await page.locator('.about-details').innerText(), /Not available yet/)
  assert.equal(
    await page.getByRole('alert').count(),
    0,
    'Native menu event listener must register successfully',
  )
  await page.getByRole('radio', { name: 'Ocean', exact: true }).check()
  await page.reload()
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByText('Connected', { exact: true }).waitFor()
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'ocean')
  for (const name of ['Drives', 'Partitions', 'USB Writer', 'Settings']) {
    await page.getByRole('navigation').getByRole('button', { name, exact: true }).click()
    assert.equal(
      await page.evaluate(
        () => globalThis.document.documentElement.scrollHeight <= globalThis.innerHeight,
      ),
      true,
      `${name} should fit without scrolling`,
    )
    assert.doesNotMatch(await page.locator('body').innerText(), /milestone/i)
  }
  await mkdir('test-results', { recursive: true })
  await page.screenshot({ path: 'test-results/native-ipc.png', fullPage: true })
  console.log(
    'PASS: native Windows UI and Rust IPC; all pages fit; Ocean theme persists; disk capabilities remain disabled.',
  )
} finally {
  await browser.close()
}
