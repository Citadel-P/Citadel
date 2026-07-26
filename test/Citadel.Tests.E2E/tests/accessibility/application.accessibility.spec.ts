import AxeBuilder from '@axe-core/playwright';
import type { Page } from '@playwright/test';

import { expect, test } from '../../support/test';

const wcagTags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'];

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({
    colorScheme: 'light',
    reducedMotion: 'reduce',
  });
});

test.describe('unauthenticated login', () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  test('login validation has no accessibility violations', async ({ page }) => {
    await page.goto('/login');
    await page.getByRole('button', { name: 'Sign in' }).click();

    await expect(
      page.getByText('Email address or username is required'),
    ).toBeVisible();
    await expect(page.getByText('Password is required')).toBeVisible();
    await expectNoAccessibilityViolations(page, 'Login validation');
  });
});

test.describe('authenticated application', () => {
  test('shell and representative resource table have no accessibility violations', async ({
    page,
  }) => {
    await page.goto('/tags');
    await expect(page.getByRole('button', { name: 'Add Tag' })).toBeVisible();

    await expectNoAccessibilityViolations(page, 'Application shell and tags table');
  });

  test('resource form with an inline error has no accessibility violations', async ({
    page,
  }) => {
    await page.goto('/stacks/add');
    await expect(page.getByRole('heading', { name: 'Add Stack' })).toBeVisible();

    const name = page.getByPlaceholder('stack-name');
    await name.fill('temporary-name');
    await name.clear();
    await expect(
      name.locator('xpath=ancestor::fieldset').getByText('Required', {
        exact: true,
      }),
    ).toBeVisible();

    await expectNoAccessibilityViolations(page, 'Stack FormBuilder validation');
  });

  test('resource dialog has no accessibility violations', async ({ page }) => {
    await page.goto('/tags');
    await page.getByRole('button', { name: 'Add Tag' }).click();

    const dialog = page.getByRole('dialog', { name: 'Create Tag' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Save' })).toBeDisabled();

    await expectNoAccessibilityViolations(page, 'Create Tag dialog');
  });
});

async function expectNoAccessibilityViolations(
  page: Page,
  surface: string,
) {
  const results = await new AxeBuilder({ page })
    .withTags(wcagTags)
    .analyze();
  const summary = results.violations.map((violation) => ({
    id: violation.id,
    impact: violation.impact,
    help: violation.help,
    targets: violation.nodes.map((node) => node.target),
  }));

  expect(
    results.violations,
    `${surface} accessibility violations:\n${JSON.stringify(summary, null, 2)}`,
  ).toEqual([]);
}
