import { test as base, expect } from '@playwright/test';

const redactUrl = (value: string) => {
  const url = new URL(value);
  for (const key of ['access_token', 'code', 'token']) {
    if (url.searchParams.has(key)) {
      url.searchParams.set(key, '[REDACTED]');
    }
  }
  return url.toString();
};

export const test = base.extend({
  page: async ({ page }, use, testInfo) => {
    const browserErrors: string[] = [];
    const failedResponses: string[] = [];

    page.on('console', (message) => {
      if (message.type() === 'error') {
        browserErrors.push(message.text());
      }
    });
    page.on('pageerror', (error) => browserErrors.push(error.message));
    page.on('response', (response) => {
      if (response.status() >= 400) {
        failedResponses.push(`${response.status()} ${redactUrl(response.url())}`);
      }
    });

    await use(page);

    if (testInfo.status !== testInfo.expectedStatus) {
      await testInfo.attach('browser-errors', {
        body: browserErrors.join('\n') || 'No browser errors captured.',
        contentType: 'text/plain',
      });
      await testInfo.attach('failed-responses', {
        body: failedResponses.join('\n') || 'No failed responses captured.',
        contentType: 'text/plain',
      });
    }
  },
});

export { expect };
