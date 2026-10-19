import { test, expect, type Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { themes } from '../src/models/themes'

async function expectDesktopFit(page: Page) {
  expect(
    await page.evaluate(() => ({
      width: document.documentElement.scrollWidth <= innerWidth,
      height: document.documentElement.scrollHeight <= innerHeight,
    })),
  ).toEqual({ width: true, height: true })
  const cutOff = await page.locator('main').evaluate((main) => {
    const bounds = [
      ...main.querySelectorAll<HTMLElement>('h2, h3, button, select, table, dd, .theme-choice'),
    ]
      .filter((el) => el.getClientRects().length)
      .filter((el) => {
        const rect = el.getBoundingClientRect()
        return rect.top < 0 || rect.bottom > innerHeight + 1 || rect.right > innerWidth + 1
      })
    return bounds.map((el) => el.textContent?.trim())
  })
  expect(cutOff).toEqual([])
  await expect(page.locator('body')).not.toContainText(/milestone/i)
  const appLabels = await page.locator('.brand, .page-heading, .about-panel').allTextContents()
  expect(appLabels.join(' ')).not.toContain('2.0')
}
async function audit(page: Page) {
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
    .analyze()
  expect(
    result.violations.map((v) => ({ id: v.id, targets: v.nodes.map((n) => n.target) })),
  ).toEqual([])
}
for (const theme of themes) {
  for (const viewport of [
    { width: 1240, height: 740 },
    { width: 800, height: 540 },
  ]) {
    test(`${theme.name}, ${viewport.width} × ${viewport.height}: every page fits and is accessible`, async ({
      page,
    }) => {
      const errors: string[] = []
      page.on('pageerror', (error) => errors.push(error.message))
      await page.setViewportSize(viewport)
      await page.emulateMedia({ colorScheme: 'light', reducedMotion: 'reduce' })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await page
        .getByRole('navigation')
        .getByRole('button', { name: 'Settings', exact: true })
        .click()
      await page.getByRole('radio', { name: theme.name, exact: true }).check()
      await page.reload()
      await expect(page.locator('html')).toHaveAttribute(
        'data-theme',
        theme.id === 'system' ? 'light' : theme.id,
      )
      for (const name of ['Drives', 'Partitions', 'USB Writer', 'Settings']) {
        await page.getByRole('navigation').getByRole('button', { name, exact: true }).click()
        await expect(page.getByRole('heading', { level: 1, name, exact: true })).toBeVisible()
        await expectDesktopFit(page)
        await audit(page)
        if (name === 'USB Writer') {
          await expect(page.getByRole('button', { name: 'Choose file' })).toBeDisabled()
          await expect(page.getByRole('button', { name: 'Write to USB' })).toBeDisabled()
        }
        if (name === 'Drives') {
          await expect(page.getByText('DEMO DATA', { exact: true })).toBeVisible()
          await page.getByRole('button', { name: 'Select Archive' }).click()
          const widths = await page
            .locator('.partition-segment')
            .evaluateAll((elements) => elements.map((el) => el.getBoundingClientRect().width))
          expect(widths[0] / (widths[0] + widths[1])).toBeCloseTo(0.9, 2)
          await expectDesktopFit(page)
          await page.getByRole('button', { name: 'Select Portable USB' }).click()
          await expectDesktopFit(page)
          await page.getByRole('button', { name: 'Select System Drive' }).click()
        }
        if (viewport.width === 1240 && theme.id !== 'system')
          await page.screenshot({
            path: `test-results/${theme.id}-${name.toLowerCase().replaceAll(' ', '-')}.png`,
          })
      }
      expect(errors).toEqual([])
    })
  }
}
test('theme picker supports the keyboard, stored choices and system appearance changes', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1024, height: 600 })
  await page.emulateMedia({ colorScheme: 'light', reducedMotion: 'reduce' })
  await page.goto('/')
  await page.waitForLoadState('networkidle')
  await page.keyboard.press('Tab')
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(page.locator('#main-content')).toBeFocused()
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click()
  await page.getByRole('radio', { name: 'System', exact: true }).focus()
  await page.keyboard.press('ArrowRight')
  await expect(page.getByRole('radio', { name: 'Light', exact: true })).toBeChecked()
  await page.getByRole('radio', { name: 'System', exact: true }).check()
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await page.getByRole('radio', { name: 'Sand', exact: true }).check()
  await page.emulateMedia({ colorScheme: 'light' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'sand')
  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'sand')
  await expectDesktopFit(page)
})
test('narrow browser previews keep navigation and controls accessible', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')
  for (const name of ['Drives', 'Partitions', 'USB Writer', 'Settings']) {
    await page.getByRole('navigation').getByRole('button', { name, exact: true }).click()
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    await audit(page)
  }
})
