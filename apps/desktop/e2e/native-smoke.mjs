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
  const inventory = await page.evaluate(() => globalThis.__TAURI_INTERNALS__.invoke('list_disks'))
  assert.ok(Array.isArray(inventory), 'Real Rust discovery must return a typed inventory')
  assert.ok(
    inventory.every(
      (d) =>
        !d.id.startsWith('demo-') &&
        typeof d.capacityBytes === 'string' &&
        BigInt(d.capacityBytes) > 0n,
    ),
  )
  assert.equal(new Set(inventory.map((d) => d.id)).size, inventory.length)
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByText('Connected', { exact: true }).waitFor()
  await page.getByLabel('Device source').selectOption('live')
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
    if (name === 'USB Writer' && inventory.length) {
      assert.equal(await page.getByLabel('USB drive').inputValue(), '')
      assert.equal(await page.getByRole('button', { name: 'Write to USB' }).isDisabled(), true)
      const options = await page.locator('#usb-target option').count()
      assert.equal(
        options,
        1 +
          inventory.filter(
            (d) =>
              d.connection?.toUpperCase() === 'USB' &&
              ['external', 'removable'].includes(d.kind) &&
              d.system === false &&
              d.online !== false &&
              d.readOnly !== true,
          ).length,
      )
    }
  }
  await page.getByLabel('Device source').selectOption('demo')
  await page.getByRole('navigation').getByRole('button', { name: 'Drives', exact: true }).click()
  await page.getByRole('button', { name: 'Select Archive' }).click()
  await page.getByText('DEMO DATA', { exact: true }).waitFor()
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByLabel('Device source').selectOption('live')
  await page.getByRole('navigation').getByRole('button', { name: 'Drives', exact: true }).click()
  await page.getByText('LIVE · READ ONLY', { exact: true }).waitFor()
  if (inventory.length) {
    const target = page.getByRole('button', { name: `Select ${inventory[0].name}`, exact: true })
    await target.click()
    await page.getByRole('button', { name: 'Refresh drives' }).click()
    await page.getByText('Live inventory refreshed', { exact: true }).waitFor()
    assert.equal(await target.getAttribute('aria-pressed'), 'true')
  }
  await mkdir('test-results', { recursive: true })
  await page.screenshot({ path: 'test-results/native-ipc.png', fullPage: true, mask: [page.locator('.device-facts dd')] })
  console.log(
    `PASS: native Windows discovery (${inventory.length} physical disks, ${inventory.reduce((n, d) => n + (d.partitions?.length ?? 0), 0)} partitions); refresh, source switching, USB eligibility, page fit and theme persistence; writes disabled.`,
  )
} finally {
  await browser.close()
}
