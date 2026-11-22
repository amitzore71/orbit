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
test('completed tasks stay visible in Today and All active, and start/end dates can be added manually if not captured', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Tasks 3', exact: true}).click();
  // Manually add start and end dates when not captured
  await page.getByRole('button', {name: 'Add dates for Read a few pages, take a few notes'}).click();
  await expect(page.getByLabel('Start date', {exact: true})).toHaveValue('');
  await expect(page.getByLabel('End date', {exact: true})).toHaveValue('');
  await page.getByLabel('Start date', {exact: true}).fill('2026-10-18');
  await page.getByLabel('End date', {exact: true}).fill('2026-10-20');
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await expect(page.getByRole('status')).toContainText('Task updated.');
  await page.getByRole('button', {name: 'Close details'}).click();
  // Complete the task and verify it stays visible in Today and All active
  await page.getByRole('button', {name: 'Complete Read a few pages, take a few notes'}).click();
  await expect(page.getByRole('button', {name: 'Reopen Read a few pages, take a few notes'})).toBeVisible();
  await page.getByRole('button', {name: 'All active', exact: true}).click();
  await expect(page.getByRole('button', {name: 'Reopen Read a few pages, take a few notes'})).toBeVisible();
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
test('titlebar maximize/restore button and double-click toggle window state', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  const app = page.locator('.orbit-app');
  await expect(app).toHaveAttribute('data-maximized', 'false');
  await page.getByRole('button', {name: 'Maximize Orbit', exact: true}).click();
  await expect(app).toHaveAttribute('data-maximized', 'true');
  await expect(page.getByRole('button', {name: 'Restore Orbit window', exact: true})).toBeVisible();
  await page.locator('header.titlebar').dblclick({position: {x: 160, y: 24}});
  await expect(app).toHaveAttribute('data-maximized', 'false');
  await expect(page.getByRole('button', {name: 'Maximize Orbit', exact: true})).toBeVisible();
});
test('focus presets and live +5m extension update timer flow', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Choose focus preset'}).click();
  await page.getByRole('dialog', {name: 'Focus presets'}).getByRole('button', {name: /Deep Work/}).click();
  await expect(page.locator('.time-display')).toHaveText('50:00');
  await page.getByRole('button', {name: /Start focusing/}).click();
  await page.getByRole('button', {name: 'Add 5 minutes'}).click();
  await expect(page.locator('.time-display')).toHaveText('55:00');
});
test('daily goal, streaks, heatmap, Zen mode, and Ctrl+K command palette work together', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await expect(page.locator('.sidebar-pulse')).toContainText('3 / 4 daily goal');
  await page.getByRole('button', {name: 'Insights', exact: true}).click();
  await expect(page.getByRole('group', {name: '12-week focus activity heatmap'})).toBeVisible();
  await expect(page.locator('.consistency-card')).toContainText('6 days');
  // Open Command Palette with Ctrl+K and enter Zen Focus Mode
  await page.keyboard.press('Control+k');
  await expect(page.getByRole('dialog', {name: 'Command palette'})).toBeVisible();
  await page.getByLabel('Type a command or task').fill('Zen');
  await page.keyboard.press('Enter');
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-zen', 'true');
  await page.keyboard.press('Escape');
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-zen', 'false');
});
test('ambient soundscape selector, Ctrl+K soundscape, and taskbar progress setting work', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Ambient focus soundscape'}).click();
  await page.getByRole('dialog', {name: 'Ambient soundscape'}).getByRole('button', {name: /Brown noise/}).click();
  await expect(page.getByRole('button', {name: 'Ambient focus soundscape'})).toContainText('Brown noise');
  await page.keyboard.press('Escape');
  await page.getByRole('button', {name: /Start focusing/}).click();
  await expect(page.locator('.ambient-trigger.is-playing')).toBeVisible();
  // Switch soundscape from Ctrl+K
  await page.keyboard.press('Control+k');
  await page.getByLabel('Type a command or task').fill('Soft rain');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', {name: 'Ambient focus soundscape'})).toContainText('Soft rain');
  // Toggle Windows taskbar progress in Settings -> Appearance
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await page.getByRole('button', {name: 'Appearance', exact: true}).click();
  const tb = page.getByRole('switch', {name: 'Windows taskbar progress'});
  await expect(tb).toHaveAttribute('aria-checked', 'true');
  await tb.click();
  await expect(tb).toHaveAttribute('aria-checked', 'false');
});
test('task tags filter the queue and Insights shows peak focus hours and tag breakdown', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await page.getByRole('button', {name: 'Tasks 3', exact: true}).click();
  await expect(page.getByRole('group', {name: 'Filter by tag'})).toBeVisible();
  await page.getByRole('group', {name: 'Filter by tag'}).getByRole('button', {name: 'Study', exact: true}).click();
  await expect(page.getByText('Read a few pages, take a few notes')).toBeVisible();
  await expect(page.getByText('Shape the next chapter of Orbit')).toHaveCount(0);
  await page.getByRole('group', {name: 'Filter by tag'}).getByRole('button', {name: 'All tags', exact: true}).click();
  // Create a tagged task
  await page.getByLabel('Task title').fill('Client architecture review');
  await page.getByLabel('Task tag').fill('Client');
  await page.getByRole('button', {name: 'Add task'}).click();
  await expect(page.getByRole('group', {name: 'Filter by tag'}).getByRole('button', {name: 'Client', exact: true})).toBeVisible();
  // Check Insights peak focus hours and Focus by tag
  await page.getByRole('button', {name: 'Insights', exact: true}).click();
  await expect(page.locator('.peak-hours-card')).toContainText('Peak: 8a–12p');
  await expect(page.locator('.tag-breakdown-card')).toContainText('Deep Work');
  await expect(page.locator('.tag-breakdown-card')).toContainText('71%');
});
