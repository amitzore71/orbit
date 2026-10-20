import { test, expect } from '@playwright/test';
import { mockDesktop } from './mock-desktop';
test('choose work, start focus, and pause from the shared workspace', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await expect(page.getByRole('heading', {name: 'One thing at a time.'})).toBeVisible();
  await page.getByRole('textbox', {name: 'Task title'}).fill('Write the first paragraph');
  await page.getByRole('button', {name: 'Add task', exact: true}).click();
  await expect(page.getByRole('button', {name: 'Write the first paragraph 0 / 1 sessions'})).toBeVisible();
  await page.getByRole('button', {name: 'Start focus on Write the first paragraph'}).click();
  await expect(page.getByRole('button', {name: /Pause session/})).toBeVisible();
  await expect(page.locator('.task-picker-trigger')).toContainText('Write the first paragraph');
  await page.getByRole('button', {name: /Pause session/}).click();
  await expect(page.getByRole('button', {name: /Resume session/})).toBeVisible();
});
test('Trash and Undo retain task identity', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Tasks 3', exact: true}).click();
  await page.getByRole('button', {name: 'Edit Shape the next chapter of Orbit'}).click();
  await page.getByRole('button', {name: 'Move to Trash'}).click();
  await expect(page.getByRole('status')).toContainText('Task moved to Trash');
  await page.getByRole('button', {name: 'Undo', exact: true}).click();
  await page.getByRole('button', {name: 'All active', exact: true}).click();
  await expect(page.getByRole('button', {name: 'Edit Shape the next chapter of Orbit'})).toBeVisible();
});
test('settings drafts survive navigation and a late save acknowledgement', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await page.getByRole('spinbutton', {name: 'Focus minutes', exact: true}).fill('35');
  await page.getByRole('button', {name: 'Focus', exact: true}).first().click();
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await expect(page.getByRole('spinbutton', {name: 'Focus minutes', exact: true})).toHaveValue('35');
  await page.evaluate(() => {(window as any).__ORBIT_LATENCY__ = 800;});
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await page.getByRole('spinbutton', {name: 'Focus minutes', exact: true}).fill('42');
  await expect(page.getByRole('status')).toContainText('Settings saved.');
  await expect(page.getByRole('spinbutton', {name: 'Focus minutes', exact: true})).toHaveValue('42');
  await expect(page.getByText('You have unsaved changes')).toBeVisible();
});
test('reports expose session details and CSV export', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Insights', exact: true}).click();
  await expect(page.getByText('4h 13m', {exact: true})).toBeVisible();
  await page.locator('.report-task').first().click();
  await expect(page.getByText('20 Oct · 09:30 · 25m · Complete')).toBeVisible();
  await page.getByRole('button', {name: 'Close details'}).click();
  await page.getByRole('button', {name: 'Export CSV'}).click();
});
test('5,000 tasks remain virtualized and searchable', async ({page}) => {
  await mockDesktop(page, {many: true}); await page.goto('/');
  await page.getByRole('button', {name: 'Tasks 5000', exact: true}).click();
  expect(await page.getByRole('listitem').count()).toBeLessThan(25);
  await page.getByRole('textbox', {name: 'Search tasks'}).fill('Research task 4999');
  await expect(page.getByRole('button', {name: 'Edit Research task 4999', exact: true})).toBeVisible();
  expect(await page.getByRole('listitem').count()).toBe(1);
});
test('storage recovery keeps controls paused until explicit resume', async ({page}) => {
  await mockDesktop(page, {blocked: true}); await page.goto('/');
  await expect(page.getByRole('button', {name: /Start focusing/})).toBeDisabled();
  await page.getByRole('button', {name: 'Retry saving'}).click();
  await expect(page.getByRole('button', {name: /Start focusing/})).toBeEnabled();
  await expect(page.getByRole('button', {name: /Pause session/})).toHaveCount(0);
});
test('the unsaved-settings exit dialog contains keyboard focus', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await expect(page.getByRole('heading',{name:'One thing at a time.'})).toBeVisible();
  await page.evaluate(()=>{(window as any).__ORBIT_EMIT__('orbit://confirm-quit',null);});
  const first=page.getByRole('button',{name:'Return to settings'});
  const last=page.getByRole('button',{name:'Discard & quit'});
  await expect(first).toBeFocused();
  await page.keyboard.press('Shift+Tab');await expect(last).toBeFocused();
  await page.keyboard.press('Tab');await expect(first).toBeFocused();
  await page.keyboard.press('Escape');await expect(page.getByRole('alertdialog')).toHaveCount(0);
});
test('long error messages stay inside the viewport', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await expect(page.getByRole('heading',{name:'One thing at a time.'})).toBeVisible();
  await page.evaluate(()=>{(window as any).__ORBIT_EMIT__('orbit://error',{message:'Ctrl+Alt+P is already in use. Choose another shortcut. Ctrl+Alt+O is already in use. Choose another shortcut.',accepted:false});});
  const toast=page.locator('.error-toast');
  await expect(toast).toBeVisible();
  await expect.poll(async()=>{const box=await toast.boundingBox();return !!box && box.x>=0 && box.x+box.width<=page.viewportSize()!.width;}).toBe(true);
});
test('visual states fit the supported window sizes and themes', async ({browser}) => {
  test.setTimeout(120000);
  for (const [label, width, height, scale, light, reduced, contrast] of [
    ['dark', 1160, 790, 1, false, false, false], ['compact', 760, 540, 1, false, false, false],
    ['light-125', 940, 740, 1.25, true, false, false], ['dark-150', 940, 740, 1.5, false, true, false],
    ['contrast-200', 940, 740, 2, false, true, true],
  ] as const) {
    const context = await browser.newContext({viewport: {width, height}, deviceScaleFactor: scale});
    const page = await context.newPage(); await mockDesktop(page, {light, reduced, contrast}); await page.goto('/');
    await expect(page.getByRole('button', {name: /Start focusing/})).toBeVisible();
    await page.waitForTimeout(450);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
    await page.screenshot({path: `../artifacts/redesign/${label}-focus.png`, fullPage: true});
    if (label === 'dark' || label === 'light-125') for (const [name, suffix] of [['Tasks 3', 'tasks'], ['Insights', 'reports'], ['Settings', 'settings']]) {
      await page.getByRole('button', {name, exact: true}).click(); await page.waitForTimeout(350);
      await page.screenshot({path: `../artifacts/redesign/${label}-${suffix}.png`, fullPage: true});
    }
    await context.close();
  }
});
