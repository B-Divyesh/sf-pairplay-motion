import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('landing page is accessible and responsive', async ({ page }, testInfo) => {
  await page.goto('/');
  await expect(page).toHaveTitle(/PairPlay Motion/);
  await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
  await expect(page.getByRole('button', { name: 'Host a game' })).toBeVisible();
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((violation) => ['serious', 'critical'].includes(violation.impact || ''))).toEqual([]);
  if (testInfo.project.name === 'mobile') {
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(overflow).toBe(false);
  }
});

test('two players can join, calibrate with fallback, and start a round', async ({ browser, page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Host a game' }).click();
  const heading = page.getByRole('heading', { name: /Room/ });
  await expect(heading).toBeVisible();
  const code = (await heading.textContent())!.replace('Room', '').trim();

  const first = await browser.newPage();
  const second = await browser.newPage();
  for (const [controller, name] of [[first, 'Ada'], [second, 'Lin']] as const) {
    await controller.goto(`/?join=${code}`);
    await controller.getByLabel('Scoreboard name').fill(name);
    await controller.getByRole('button', { name: 'Join room' }).click();
    await expect(controller.getByText('Connected. Look at the host screen.')).toBeVisible();
  }
  await expect(page.getByText('2 of 4')).toBeVisible();
  await page.getByRole('button', { name: 'Calibrate all phones' }).click();
  await first.getByRole('button', { name: 'Use touch controls instead' }).click();
  await second.getByRole('button', { name: 'Use touch controls instead' }).click();
  await expect(page.getByText('✓ Calibrated')).toHaveCount(2);
  await page.getByRole('button', { name: 'Play Dead Still' }).click();
  await expect(page.getByText(/MOVE|FREEZE/, { exact: true })).toBeVisible();
  await expect(first.locator('.phone-score')).toContainText('Score');
  await first.close(); await second.close();
});

test('privacy and terms have real routes', async ({ page }) => {
  await page.goto('/privacy');
  await expect(page.getByRole('heading', { name: 'Privacy, in plain language' })).toBeVisible();
  await page.goto('/terms');
  await expect(page.getByRole('heading', { name: 'Terms of play' })).toBeVisible();
});
