import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';

test.describe.configure({ mode: 'serial' });

async function hostFromSample(page: import('@playwright/test').Page) {
  await page.goto('/demo');
  await page.getByRole('button', { name: 'Start for real' }).click();
  await page.getByRole('button', { name: 'Host a game' }).click();
  await expect(page.getByText(/LIVE ROOM/)).toBeVisible();
  const heading = page.getByRole('heading', { level: 1, name: /Room/ });
  const code = (await heading.textContent())!.replace('Room', '').trim();
  return code;
}

async function joinController(browser: import('@playwright/test').Browser, code: string, name: string) {
  const controller = await browser.newPage();
  await controller.goto(`/?join=${code}`);
  await controller.getByLabel('Scoreboard name').fill(name);
  await controller.getByRole('button', { name: 'Join room' }).click();
  await expect(controller.getByText('Connected. Look at the host screen.')).toBeVisible();
  return controller;
}

function databaseSnapshot() {
  const script = [
    'import json, sqlite3, sys',
    'db = sqlite3.connect(sys.argv[1])',
    "tables = [row[0] for row in db.execute(\"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name\")]",
    "columns = [row[1] for row in db.execute(\"PRAGMA table_info(page_views)\")]",
    "views = db.execute(\"SELECT COALESCE(SUM(views), 0) FROM page_views\").fetchone()[0]",
    "print(json.dumps({'tables': tables, 'columns': columns, 'views': views}))",
  ].join('\n');
  return JSON.parse(execFileSync('python3', ['-c', script, 'data/pairplay.db'], { encoding: 'utf8' })) as {
    tables: string[]; columns: string[]; views: number;
  };
}

test('@claim:no-app Sample games run in the browser without an app download', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();
  await expect(page.getByRole('heading', { level: 1, name: /Room SAMPLE/ })).toBeVisible();
  await expect(page.locator('a[href*="appstore"], a[href*="play.google"]')).toHaveCount(0);
});

test('@claim:no-account The sample opens without creating an account', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByRole('heading', { level: 1, name: /Room SAMPLE/ })).toBeVisible();
  await expect(page.locator('input[type="password"], input[autocomplete="username"]')).toHaveCount(0);
  await expect(page).toHaveURL(/\/demo$/);
});

test('@claim:demo-sandbox Sample data is populated, isolated, and resettable', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('pairplay:real-note', 'keep'));
  await page.goto('/demo');
  await expect(page.getByText('Ada', { exact: true })).toBeVisible();
  await expect(page.getByText('34', { exact: true })).toBeVisible();
  await expect(page.getByText('Lin', { exact: true })).toBeVisible();
  await expect(page.getByText('28', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play again' }).click();
  await expect(page.getByText('MOVE', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByText('Round results', { exact: true })).toBeVisible();
  await expect(page.getByText('34', { exact: true })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('pairplay:real-note'))).toBe('keep');
  expect(await page.evaluate(() => sessionStorage.getItem('demo:pairplay-motion'))).toBe('loaded');
  await page.getByRole('button', { name: 'Start for real' }).click();
  expect(await page.evaluate(() => sessionStorage.getItem('demo:pairplay-motion'))).toBeNull();
  expect(await page.evaluate(() => localStorage.getItem('pairplay:real-note'))).toBe('keep');
});

test('@claim:sensor-retention Motion controls do not write samples to browser storage', async ({ browser, page }) => {
  const code = await hostFromSample(page);
  const first = await joinController(browser, code, 'Ada');
  const second = await joinController(browser, code, 'Lin');
  await page.getByRole('button', { name: 'Calibrate phones' }).click();
  await first.getByRole('button', { name: 'Use touch controls instead' }).click();
  await second.getByRole('button', { name: 'Use touch controls instead' }).click();
  await first.getByRole('button', { name: 'Tilt left' }).click();
  const storage = await first.evaluate(() => [...Object.keys(localStorage), ...Object.keys(sessionStorage)]);
  expect(storage.filter((key) => /motion|sensor|sample/i.test(key))).toEqual([]);
  await first.close();
  await second.close();
});

test('@claim:two-to-four-phone-support A room admits four players and refuses a fifth', async ({ browser, page }) => {
  const code = await hostFromSample(page);
  const controllers = [];
  for (const name of ['Ada', 'Lin', 'Moe', 'Rae']) controllers.push(await joinController(browser, code, name));
  await expect(page.getByText('Players / 4 of 4')).toBeVisible();
  const fifth = await browser.newPage();
  await fifth.goto(`/?join=${code}`);
  await fifth.getByLabel('Scoreboard name').fill('Five');
  await fifth.getByRole('button', { name: 'Join room' }).click();
  await expect(fifth.getByText('This room already has four players.')).toBeVisible();
  await Promise.all([...controllers, fifth].map((controller) => controller.close()));
});

test('@claim:free-dead-still The free game starts from the populated sample', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByText(/Dead Still/)).toBeVisible();
  await page.getByRole('button', { name: 'Play again' }).click();
  await expect(page.getByText('MOVE', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'End round now' }).click();
  await expect(page.getByText('Round results', { exact: true })).toBeVisible();
});

test('@claim:price The paid games have an actionable US $8 offer', async ({ page }) => {
  await page.goto('/demo');
  await page.getByRole('button', { name: 'Start for real' }).click();
  await expect(page.getByRole('heading', { name: 'Three games for US $8 once' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Buy the full edition' })).toHaveAttribute('href', /\/api\/v1\/products\/pairplay-motion\/checkout$/);
});

test('@claim:one-time-license The offer states a one-time license and no subscription', async ({ page }) => {
  await page.goto('/demo');
  await page.getByRole('button', { name: 'Start for real' }).click();
  await expect(page.getByText('A one-time license adds News Desk and Ink Runner. There is no subscription.')).toBeVisible();
});

test('@claim:license-storage A returned license token is kept in this browser and removed from the URL', async ({ page }) => {
  await page.goto('/demo');
  await page.getByRole('button', { name: 'Start for real' }).click();
  await page.route('https://api.sociobot.in/**', (route) => route.abort());
  await page.goto('/?license=sample-license-token');
  await expect(page).not.toHaveURL(/license=/);
  expect(await page.evaluate(() => localStorage.getItem('sb_license:pairplay-motion'))).toBe('sample-license-token');
});

test('@claim:touch-keyboard-fallback Touch calibration and keyboard input reach a live round', async ({ browser, page }) => {
  const code = await hostFromSample(page);
  const first = await browser.newPage();
  const frames: string[] = [];
  first.on('websocket', (socket) => socket.on('framesent', (frame) => {
    if (typeof frame.payload === 'string') frames.push(frame.payload);
  }));
  await first.goto(`/?join=${code}`);
  await first.getByLabel('Scoreboard name').fill('Ada');
  await first.getByRole('button', { name: 'Join room' }).click();
  await expect(first.getByText('Connected. Look at the host screen.')).toBeVisible();
  const second = await joinController(browser, code, 'Lin');
  await page.getByRole('button', { name: 'Calibrate phones' }).click();
  await first.getByRole('button', { name: 'Use touch controls instead' }).click();
  await second.getByRole('button', { name: 'Use touch controls instead' }).click();
  await expect(page.getByText('Calibrated', { exact: true })).toHaveCount(2);
  await first.keyboard.press('ArrowLeft');
  await first.keyboard.press('Space');
  await expect.poll(() => frames.some((frame) => frame.includes('"x":-24'))).toBe(true);
  await expect.poll(() => frames.some((frame) => frame.includes('"shake":10'))).toBe(true);
  await page.getByRole('button', { name: 'Play Dead Still' }).click();
  await expect(page.getByText(/MOVE|FREEZE/, { exact: true })).toBeVisible();
  await first.close();
  await second.close();
});

test('@claim:server-memory Rooms and motion are absent from the durable SQLite database', async ({ page }) => {
  await page.goto('/demo');
  await page.request.post('/api/page-view');
  const snapshot = databaseSnapshot();
  expect(snapshot.tables).toEqual(['page_views']);
  expect(snapshot.columns).toEqual(['day', 'views']);
});

test('@claim:aggregate-page-count A page view changes only the daily aggregate count', async ({ page }) => {
  await page.goto('/demo');
  const before = databaseSnapshot();
  const response = await page.request.post('/api/page-view');
  expect(response.status()).toBe(204);
  const after = databaseSnapshot();
  expect(after.views).toBe(before.views + 1);
  expect(after.tables).toEqual(['page_views']);
});

test('@claim:no-trackers The sample makes only same-origin requests and no page-view write', async ({ page }) => {
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));
  await page.goto('/demo');
  await page.waitForLoadState('networkidle');
  const urls = requests.map((url) => new URL(url));
  expect(urls.every((url) => url.origin === 'http://127.0.0.1:8080')).toBe(true);
  expect(urls.some((url) => url.pathname === '/api/page-view')).toBe(false);
  expect(urls.some((url) => url.pathname === '/ws')).toBe(false);
});

test('@claim:rate-limits Room creation returns 429 and Retry-After for a limited forwarded client', async ({ page }) => {
  await page.goto('/demo');
  const headers = { 'x-forwarded-for': '198.51.100.80, 10.0.0.2' };
  for (let count = 0; count < 12; count += 1) {
    const response = await page.request.post('/api/rooms', { headers, data: {} });
    expect(response.status()).toBe(200);
  }
  const limited = await page.request.post('/api/rooms', { headers, data: {} });
  expect(limited.status()).toBe(429);
  expect(limited.headers()['retry-after']).toBe('5');
  const other = await page.request.post('/api/rooms', { headers: { 'x-forwarded-for': '203.0.113.81' }, data: {} });
  expect(other.status()).toBe(200);
});
