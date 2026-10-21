import { test, expect } from '@playwright/test';
import { mockDesktop } from './mock-desktop';

async function appearance(page: import('@playwright/test').Page) {
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await page.getByRole('button', {name: 'Appearance', exact: true}).click();
}

test('appearance previews, discard, save, and reset respect the draft boundary', async ({page}) => {
  await mockDesktop(page); await page.goto('/'); await appearance(page);
  await page.getByRole('button', {name: 'Sage', exact: true}).click();
  await page.getByLabel('Interface density').selectOption('compact');
  await page.getByLabel('Animation level').selectOption('off');
  await page.getByRole('button', {name: 'Light', exact: true}).click();
  const app = page.locator('.orbit-app');
  await expect(app).toHaveAttribute('data-accent', 'sage');
  await expect(app).toHaveAttribute('data-density', 'compact');
  await expect(app).toHaveAttribute('data-motion', 'off');
  await expect(app).toHaveAttribute('data-theme', 'light');
  expect(await page.evaluate(() => (window as any).__ORBIT_CALLS__.filter((c: any) => c.args?.command?.type === 'save_settings').length)).toBe(0);
  await page.getByRole('button', {name: 'Discard', exact: true}).click();
  await expect(app).toHaveAttribute('data-accent', 'coral');
  await expect(app).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('button', {name: 'Slate Blue', exact: true}).click();
  await page.getByLabel('Animation level').selectOption('reduced');
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await expect(page.getByText('All changes saved', {exact: true})).toBeVisible();
  await page.getByRole('button', {name: 'Reset appearance'}).click();
  await expect(app).toHaveAttribute('data-accent', 'coral');
  await page.getByRole('button', {name: 'Discard', exact: true}).click();
  await expect(app).toHaveAttribute('data-accent', 'slate_blue');
  await expect(app).toHaveAttribute('data-motion', 'reduced');
});

test('filters, search, list scroll, settings category, and report period survive navigation', async ({page}) => {
  await mockDesktop(page, {many: true}); await page.goto('/');
  await page.getByRole('button', {name: 'Tasks 5000', exact: true}).click();
  await page.getByRole('button', {name: 'All active', exact: true}).click();
  await page.getByLabel('Search tasks').fill('Research task');
  await page.getByRole('list', {name: 'Tasks'}).evaluate(e => {e.scrollTop = 4200;});
  await expect.poll(() => page.getByRole('list', {name: 'Tasks'}).evaluate(e => e.scrollTop)).toBeGreaterThan(4000);
  await page.getByRole('button', {name: 'Insights', exact: true}).click();
  await page.getByRole('button', {name: 'Monthly', exact: true}).click();
  await appearance(page);
  await page.getByRole('button', {name: 'Tasks 5000', exact: true}).click();
  await expect(page.getByLabel('Search tasks')).toHaveValue('Research task');
  await expect(page.getByRole('button', {name: 'All active', exact: true})).toHaveAttribute('aria-pressed', 'true');
  await expect.poll(() => page.getByRole('list', {name: 'Tasks'}).evaluate(e => e.scrollTop)).toBeGreaterThan(4000);
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await expect(page.getByLabel('Animation level')).toBeVisible();
  await page.getByRole('button', {name: 'Insights', exact: true}).click();
  await expect(page.getByRole('button', {name: 'Monthly', exact: true})).toHaveAttribute('aria-pressed', 'true');
});

test('picker and details close on Escape and restore keyboard focus', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  const trigger = page.locator('.task-picker-trigger');
  await trigger.click();
  await expect(page.getByLabel('Find a focus task')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
  const edit = page.getByRole('button', {name: 'Edit Shape the next chapter of Orbit'});
  await edit.click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(edit).toBeFocused();
});

test('5,000 tasks support compact density and keyboard access to the last row', async ({page}) => {
  await mockDesktop(page, {many: true}); await page.goto('/'); await appearance(page);
  await page.getByLabel('Interface density').selectOption('compact');
  await page.getByRole('button', {name: 'Tasks 5000', exact: true}).click();
  const list = page.getByRole('list', {name: 'Tasks'});
  await list.focus(); await page.keyboard.press('End');
  await expect(page.getByRole('button', {name: 'Research task 5000 1 / 3 sessions'})).toBeFocused();
  expect(await page.getByRole('listitem').count()).toBeLessThan(25);
  await expect(page.getByRole('listitem').last()).toHaveCSS('height', '68px');
  await page.keyboard.press('Home');
  await expect(page.getByRole('button', {name: 'Research task 1 1 / 3 sessions'})).toBeFocused();
});

test('a rejected appearance save retains its draft and an accepted failure uses retry', async ({page}) => {
  await mockDesktop(page); await page.goto('/'); await appearance(page);
  await page.getByRole('button', {name: 'Amber', exact: true}).click();
  await page.evaluate(() => {(window as any).__ORBIT_SAVE_ERROR__ = 'rejected';});
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await expect(page.locator('.error-toast')).toContainText('Settings could not be saved');
  await expect(page.getByText('You have unsaved changes', {exact: true})).toBeVisible();
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-accent', 'amber');
  await page.evaluate(() => {(window as any).__ORBIT_SAVE_ERROR__ = 'accepted';});
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await expect(page.getByRole('button', {name: 'Retry saving'})).toBeVisible();
  await page.getByRole('button', {name: 'Retry saving'}).click();
  await expect(page.getByRole('button', {name: 'Retry saving'})).toHaveCount(0);
  const saves = await page.evaluate(() => (window as any).__ORBIT_CALLS__.filter((c: any) => c.args?.command?.type === 'save_settings').length);
  expect(saves).toBe(2);
});

test('rapid navigation lands on the latest page and uses the configured Windows shortcut', async ({page}) => {
  await mockDesktop(page); await page.goto('/');
  await expect(page.locator('.timer-controls kbd')).toHaveText('Ctrl+Alt+P');
  for (const name of ['Insights', 'Tasks 3', 'Settings', 'Focus']) await page.getByRole('button', {name, exact: true}).first().click();
  await expect(page.getByRole('heading', {name: 'One thing at a time.'})).toBeVisible();
  await expect(page.locator('.focus-workspace')).toHaveCount(1);
});

test('compact windows keep the countdown and primary control inside the first viewport', async ({page}) => {
  await page.setViewportSize({width: 760, height: 540});
  await mockDesktop(page); await page.goto('/');
  await expect(page.locator('.time-display')).toBeInViewport();
  await expect(page.getByRole('button', {name: 'Start focusing', exact: true})).toBeInViewport();
});

test('each widget keeps its primary controls and task summary inside its native bounds in both densities', async ({page}) => {
  await mockDesktop(page);
  for (const [kind, width, height] of [[0, 300, 360], [1, 340, 390], [2, 310, 116]]) {
    await page.setViewportSize({width, height});
    await page.goto(`/widget?kind=${kind}`);
    await expect(page.getByRole('button', {name: 'Close widget'})).toBeVisible();
    for (const density of ['comfortable', 'compact']) {
      await page.evaluate(async density => {
        const bridge = (window as any).__ORBIT_TEST__;
        const snapshot = await bridge.invoke('orbit_command', {command: {type: 'snapshot'}});
        await bridge.invoke('orbit_command', {command: {type: 'save_settings', settings: {...snapshot.value.settings, density}}});
      }, density);
      await expect(page.locator('.orbit-app')).toHaveAttribute('data-density', density);
      if (kind === 1) {
        await expect(page.locator('.view-all')).toBeInViewport({ratio: 1});
        await expect(page.locator('.today-totals')).toBeInViewport({ratio: 1});
      } else await expect(page.getByRole('button', {name: 'Start focusing', exact: true})).toBeInViewport({ratio: 1});
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await page.screenshot({path: `../artifacts/upgrade/widget-${kind}-${density}.png`});
    }
  }
});

test('a fresh empty task widget keeps its add control and totals visible', async ({page}) => {
  await mockDesktop(page); await page.setViewportSize({width: 340, height: 390});
  await page.goto('/widget?kind=1');
  await expect(page.getByRole('button', {name: 'Close widget'})).toBeVisible();
  await page.evaluate(async () => {
    const bridge = (window as any).__ORBIT_TEST__;
    const snapshot = await bridge.invoke('orbit_command', {command: {type: 'snapshot'}});
    for (const task of snapshot.value.tasks) await bridge.invoke('orbit_command', {command: {type: 'task', id: task.id, action: 'delete'}});
  });
  await expect(page.getByRole('heading', {name: 'A fresh page for today.'})).toBeVisible();
  for (const density of ['comfortable', 'compact']) {
    await page.evaluate(async density => {
      const bridge = (window as any).__ORBIT_TEST__;
      const snapshot = await bridge.invoke('orbit_command', {command: {type: 'snapshot'}});
      await bridge.invoke('orbit_command', {command: {type: 'save_settings', settings: {...snapshot.value.settings, density}}});
    }, density);
    await expect(page.locator('.orbit-app')).toHaveAttribute('data-density', density);
    await expect(page.getByLabel('Task title')).toBeInViewport({ratio: 1});
    await expect(page.locator('.view-all')).toBeInViewport({ratio: 1});
    await expect(page.locator('.today-totals')).toBeInViewport({ratio: 1});
    await page.screenshot({path: `../artifacts/upgrade/widget-empty-${density}.png`});
  }
});

test('all themes, accents, densities, and motion levels fit; accessibility overrides previews', async ({page}) => {
  test.setTimeout(120000);
  await mockDesktop(page); await page.goto('/'); await appearance(page);
  for (const theme of ['Light', 'Dark']) {
    await page.getByRole('button', {name: theme, exact: true}).click();
    for (const accent of ['Coral', 'Sage', 'Amber', 'Slate Blue']) {
      await page.getByRole('button', {name: accent, exact: true}).click();
      const contrast = await page.evaluate(() => {
        const app = document.querySelector('.orbit-app')!;
        const css = getComputedStyle(app);
        const luminance = (hex: string) => {
          const channels = hex.trim().slice(1).match(/../g)!.map(v => parseInt(v, 16) / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4);
          return channels[0] * .2126 + channels[1] * .7152 + channels[2] * .0722;
        };
        const ratio = (a: string, b: string) => {const x = luminance(css.getPropertyValue(a)); const y = luminance(css.getPropertyValue(b)); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05);};
        return [ratio('--ink', '--canvas'), ratio('--muted', '--surface'), ratio('--accent-ink', '--accent'), ratio('--accent-ink', '--accent-soft')];
      });
      for (const ratio of contrast) expect(ratio).toBeGreaterThanOrEqual(4.5);
      for (const density of ['comfortable', 'compact']) {
        await page.getByLabel('Interface density').selectOption(density);
        for (const motion of ['standard', 'reduced', 'off']) {
          await page.getByLabel('Animation level').selectOption(motion);
          await expect(page.locator('.orbit-app')).toHaveAttribute('data-motion', motion);
          expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        }
      }
    }
    await page.screenshot({path: `../artifacts/upgrade/${theme.toLowerCase()}-appearance.png`, fullPage: true});
  }
  await page.getByLabel('Animation level').selectOption('standard');
  await page.emulateMedia({reducedMotion: 'reduce'});
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-motion', 'reduced');
  await page.evaluate(() => {(window as any).__ORBIT_EMIT__('orbit://appearance', {dark: true, reduced_motion: true, high_contrast: true});});
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-theme', 'contrast');
  await expect(page.locator('.orbit-app')).toHaveAttribute('data-motion', 'reduced');
});
