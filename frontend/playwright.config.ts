import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests', testMatch: '**/*.e2e.ts', fullyParallel: false,
  use: {baseURL: process.env.ORBIT_PREVIEW_URL || 'http://127.0.0.1:3000', viewport: {width: 1160, height: 790},
    launchOptions: process.env.ORBIT_CHROMIUM ? {executablePath: process.env.ORBIT_CHROMIUM} : {},
    trace: 'retain-on-failure'},
  reporter: 'list', timeout: 30000,
  webServer: process.env.ORBIT_PREVIEW_URL ? undefined : {command: 'pnpm dev', url: 'http://127.0.0.1:3000', reuseExistingServer: true, timeout: 60000},
});
