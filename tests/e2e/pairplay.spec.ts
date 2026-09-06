import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('landing page is accessible and responsive', async ({ page }, testInfo) => {
  const externalRequests: string[] = [];
  page.on('request', (request) => {
    const requestUrl = new URL(request.url());
    if (requestUrl.protocol.startsWith('http') && requestUrl.origin !== 'http://127.0.0.1:8080') externalRequests.push(request.url());
  });
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await expect(page).toHaveTitle('PairPlay Motion — phone motion games');
  await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
  await expect(page.getByRole('heading', { level: 1, name: 'Turn phones into motion controllers' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Try it with sample data' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Host a game' })).toBeVisible();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeFocused();
  const focusStyle = await page.getByRole('link', { name: 'Skip to main content' }).evaluate((element) => getComputedStyle(element).outlineStyle);
  expect(focusStyle).toBe('solid');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const reducedDuration = await page.getByRole('button', { name: 'Host a game' }).evaluate((element) => Number.parseFloat(getComputedStyle(element).transitionDuration));
  expect(reducedDuration).toBeLessThanOrEqual(0.00001);
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((violation) => ['serious', 'critical'].includes(violation.impact || ''))).toEqual([]);
  expect(externalRequests).toEqual([]);
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
  const keyboardFrames: string[] = [];
  first.on('websocket', (websocket) => websocket.on('framesent', (frame) => {
    if (typeof frame.payload === 'string') keyboardFrames.push(frame.payload);
  }));
  for (const [controller, name] of [[first, 'Ada'], [second, 'Lin']] as const) {
    await controller.goto(`/?join=${code}`);
    await controller.getByLabel('Scoreboard name').fill(name);
    await controller.getByRole('button', { name: 'Join room' }).click();
    await expect(controller.getByText('Connected. Look at the host screen.')).toBeVisible();
  }
  await expect(page.getByText('2 of 4')).toBeVisible();
  await page.getByRole('button', { name: 'Calibrate phones' }).click();
  await first.getByRole('button', { name: 'Use touch controls instead' }).click();
  await second.getByRole('button', { name: 'Use touch controls instead' }).click();
  await expect(page.getByText('Calibrated', { exact: true })).toHaveCount(2);
  keyboardFrames.length = 0;
  await first.keyboard.press('ArrowLeft');
  await first.keyboard.press('Space');
  await expect.poll(() => keyboardFrames.some((frame) => frame.includes('"x":-24'))).toBe(true);
  await expect.poll(() => keyboardFrames.some((frame) => frame.includes('"shake":10'))).toBe(true);
  await page.getByRole('button', { name: 'Play Dead Still' }).click();
  await expect(page.getByText(/MOVE|FREEZE/, { exact: true })).toBeVisible();
  await expect(first.locator('.phone-score')).toContainText('Score');
  await first.close(); await second.close();
});

test('a fifth controller is told that the room is full', async ({ browser, page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Host a game' }).click();
  const code = (await page.getByRole('heading', { name: /Room/ }).textContent())!.replace('Room', '').trim();
  const controllers = await Promise.all(['Ada', 'Lin', 'Moe', 'Rae'].map(async (name) => {
    const controller = await browser.newPage();
    await controller.goto(`/?join=${code}`);
    await controller.getByLabel('Scoreboard name').fill(name);
    await controller.getByRole('button', { name: 'Join room' }).click();
    await expect(controller.getByText('Connected. Look at the host screen.')).toBeVisible();
    return controller;
  }));
  const fifth = await browser.newPage();
  await fifth.goto(`/?join=${code}`);
  await fifth.getByLabel('Scoreboard name').fill('Five');
  await fifth.getByRole('button', { name: 'Join room' }).click();
  await expect(fifth.getByText('This room already has four players.')).toBeVisible();
  await Promise.all([...controllers, fifth].map((controller) => controller.close()));
});

test('a controller gets a specific recovery message for a missing room', async ({ page }) => {
  await page.goto('/?join=ABC234');
  await page.getByLabel('Scoreboard name').fill('Ada');
  await page.getByRole('button', { name: 'Join room' }).click();
  await expect(page.getByText('Room not found. Check the code with the host.')).toBeVisible();
});

test('privacy and terms have real routes', async ({ page }) => {
  await page.goto('/privacy');
  await expect(page).toHaveTitle('Privacy — PairPlay Motion');
  await expect(page.getByRole('heading', { level: 1, name: 'Privacy' })).toBeVisible();
  await page.goto('/terms');
  await expect(page).toHaveTitle('Terms — PairPlay Motion');
  await expect(page.getByRole('heading', { level: 1, name: 'Terms' })).toBeVisible();
});

test('demo and unknown routes have their own titles and a usable 404 page', async ({ page }) => {
  await page.goto('/demo');
  await expect(page).toHaveTitle('Demo — PairPlay Motion');
  await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();
  const response = await page.goto('/missing-page');
  expect(response?.status()).toBe(404);
  await expect(page).toHaveTitle('Page not found — PairPlay Motion');
  await expect(page.getByRole('heading', { level: 1, name: 'Page not found' })).toBeVisible();
  await page.getByRole('button', { name: 'Go to the home page' }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Turn phones into motion controllers' })).toBeVisible();
});

test('invalid join input is announced once', async ({ page }) => {
  await page.goto('/?join=X');
  await page.getByLabel('Scoreboard name').fill('Ada');
  await page.getByRole('button', { name: 'Join room' }).click();
  await expect(page.getByRole('alert')).toHaveCount(1);
  await expect(page.getByRole('alert')).toContainText('Enter the six-character room code.');
});

test('installed shell reloads with an offline state', async ({ page, context }) => {
  await page.goto('/');
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  const updateEvidence = await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.ready;
    const response = await fetch('/sw.js', { cache: 'reload' });
    const source = await response.text();
    return {
      activeScript: registration.active?.scriptURL,
      cacheControl: response.headers.get('cache-control'),
      cacheNames: await caches.keys(),
      claimsClients: source.includes('clients.claim()'),
      skipsWaiting: source.includes('skipWaiting()'),
    };
  });
  expect(updateEvidence.activeScript).toMatch(/\/sw\.js$/);
  expect(updateEvidence.cacheControl).toBe('no-cache');
  expect(updateEvidence.cacheNames).toContain('pairplay-shell-v3');
  expect(updateEvidence.claimsClients).toBe(true);
  expect(updateEvidence.skipsWaiting).toBe(true);
  await context.setOffline(true);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.getByText('OFFLINE', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Host a game' })).toBeVisible();
  await context.setOffline(false);
});
