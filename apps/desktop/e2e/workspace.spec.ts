import { test, expect } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

for (const theme of ['light', 'dark'] as const) {
  for (const width of [1240, 800, 390]) {
    test(`${theme} theme at ${width}px: navigation, layout and accessibility`, async ({ page }) => {
      const errors: string[] = []
      page.on('pageerror', (error) => errors.push(error.message))
      await page.setViewportSize({ width, height: 900 })
      await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' })
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await expect(page.getByRole('heading', { name: 'Disk Overview', exact: true })).toBeVisible()
      await expect(page.getByText('DEMO DATA / SIMULATED DEVICES')).toBeVisible()
      await page.keyboard.press('Tab')
      await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused()
      await page.keyboard.press('Enter')
      await expect(page.locator('#main-content')).toBeFocused()
      await expect(page.getByRole('button', { name: 'Select System Drive' })).toHaveCSS(
        'transition-duration',
        '0s',
      )
      await page.getByRole('button', { name: 'Select Archive' }).click()
      await expect(page.getByRole('region', { name: 'Selected device' })).toContainText(
        'Crucial MX500',
      )
      const segments = await page
        .locator('.partition-segment')
        .evaluateAll((elements) => elements.map((el) => el.getBoundingClientRect().width))
      expect(segments[0] / (segments[0] + segments[1])).toBeCloseTo(0.9, 2)
      for (const name of ['Disk Overview', 'Partitions', 'USB Writer', 'Settings']) {
        await page.getByRole('navigation').getByRole('button', { name, exact: true }).click()
        await expect(page.getByRole('heading', { level: 1, name, exact: true })).toBeVisible()
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
          true,
        )
        const audit = await new AxeBuilder({ page })
          .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
          .analyze()
        expect(
          audit.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target) })),
        ).toEqual([])
      }
      await page
        .getByRole('button', { name: theme === 'light' ? 'Dark' : 'Light', exact: true })
        .click()
      await page.reload()
      await expect(page.locator('html')).toHaveClass(theme === 'light' ? /dark/ : /^$/)
      await page.screenshot({
        path: `test-results/workspace-${theme}-${width}.png`,
        fullPage: true,
      })
      expect(errors).toEqual([])
    })
  }
}
