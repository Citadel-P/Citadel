import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDirectory = path.dirname(fileURLToPath(import.meta.url));
const baseURL = process.env.CITADEL_E2E_BASE_URL ?? 'http://127.0.0.1:18000';
const visualTests = /[\\/]visual[\\/]/;

export default defineConfig({
  testDir: path.join(rootDirectory, 'tests'),
  outputDir: path.join(rootDirectory, 'test-results'),
  globalSetup: path.join(rootDirectory, 'support/global-setup.ts'),
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  timeout: 30_000,
  expect: {
    timeout: 10_000,
    toHaveScreenshot: {
      animations: 'disabled',
      caret: 'hide',
      maxDiffPixelRatio: 0.005,
      scale: 'css',
    },
  },
  reporter: [
    ['list'],
    ['html', { outputFolder: path.join(rootDirectory, 'playwright-report'), open: 'never' }],
    ['junit', { outputFile: path.join(rootDirectory, 'test-results/results.xml') }],
  ],
  use: {
    baseURL,
    storageState: path.join(rootDirectory, '.auth/admin.json'),
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      testIgnore: visualTests,
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          args: ['--host-resolver-rules=MAP keycloak 127.0.0.1'],
        },
      },
    },
    {
      name: 'visual-desktop',
      testMatch: visualTests,
      use: {
        ...devices['Desktop Chrome'],
        colorScheme: 'light',
        viewport: { width: 1440, height: 900 },
      },
    },
    {
      name: 'visual-mobile',
      testMatch: visualTests,
      use: {
        ...devices['Pixel 7'],
        colorScheme: 'light',
      },
    },
  ],
});
