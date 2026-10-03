import type { APIRequestContext, Locator, Page } from '@playwright/test';

import { deleteTagsByPrefix, getAdminAccessToken } from '../../support/admin-api';
import {
  cleanupRuntimeFixture,
  createRuntimeFixture,
  type RuntimeFixture,
} from '../../support/runtime-api';
import { expect, test } from '../../support/test';

const visualTagPrefix = 'e2e-visual-';
const visualTags = [
  { name: `${visualTagPrefix}production`, color: '#2563EB' },
  { name: `${visualTagPrefix}protected`, color: '#DC2626' },
  { name: `${visualTagPrefix}automation`, color: '#16A34A' },
];

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light', reducedMotion: 'reduce' });
  await page.addInitScript(() => {
    window.localStorage.setItem('theme', JSON.stringify({ color: 'blue', mode: 'light' }));
    window.localStorage.setItem('sidebarStatus', JSON.stringify({ minimized: false }));
  });
});

test('authenticated shell and representative resource table', async ({ page, request }) => {
  await seedVisualTags(request);

  try {
    await page.goto('/tags');
    await expect(page.getByRole('button', { name: 'Add Tag' })).toBeVisible();
    await expect(page.getByText(`${visualTagPrefix}production`, { exact: true })).toBeVisible();

    await screenshot(page, 'resource-table.png', [
      tagUpdatedAtValues(page),
    ]);
  } finally {
    await deleteTagsByPrefix(request, visualTagPrefix);
  }
});

test('collapsed desktop sidebar', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'visual-desktop', 'The mobile sidebar uses an off-canvas sheet.');

  await page.goto('/tags');
  await page.locator('[data-sidebar="trigger"]').click();
  await expect(page.locator('[data-slot="sidebar-wrapper"]')).toHaveAttribute('data-state', 'collapsed');

  await screenshot(page, 'collapsed-sidebar.png', [tagUpdatedAtValues(page)]);
});

test('representative FormBuilder form', async ({ page }) => {
  await page.goto('/stacks/add');
  await expect(page.getByRole('heading', { name: 'Add Stack' })).toBeVisible();
  await expect(page.getByText('Internal identifier for this workload.')).toBeVisible();

  await screenshot(page, 'stack-form-builder.png');
});

test('global search result dialog', async ({ page }) => {
  await page.route('**/api/v1/search?**', async (route) => {
    await route.fulfill({
      json: {
        query: 'visual',
        groups: [
          {
            category: 'Stacks',
            items: [
              {
                id: '019f98ce-8f1d-7a86-9930-c46d5ecad001',
                resourceType: 'Stack',
                name: 'payments-production',
                secondaryText: 'Production application stack',
                status: { label: 'Healthy', tone: 'Positive' },
                parent: {
                  id: '019f98ce-8f1d-7a86-9930-c46d5ecad002',
                  resourceType: 'Platform',
                  name: 'primary-docker',
                },
              },
            ],
          },
          {
            category: 'Builds',
            items: [
              {
                id: '019f98ce-8f1d-7a86-9930-c46d5ecad003',
                resourceType: 'Build',
                name: 'payments-image',
                secondaryText: 'main / src/Payments.Api',
                status: { label: 'Idle', tone: 'Neutral' },
                parent: null,
              },
            ],
          },
        ],
      },
    });
  });

  await page.goto('/tags');
  await page.getByRole('button', { name: 'Search resources' }).click();
  const dialog = page.getByRole('dialog', { name: 'Search resources' });
  await dialog.getByPlaceholder('Search resources...').fill('visual');
  await expect(dialog.getByText('payments-production', { exact: true })).toBeVisible();
  await expect(dialog.getByText('payments-image', { exact: true })).toBeVisible();

  await screenshot(page, 'global-search.png', [tagUpdatedAtValues(page)]);
});

test('completed stack task sheet', async ({ page, request }) => {
  test.setTimeout(60_000);
  let fixture: RuntimeFixture | undefined;

  try {
    fixture = await createRuntimeFixture(request, 'visual-task', 'EdgeAgent');
    await page.route('**/api/v1/stacks/apply', async (route) => {
      await route.fulfill({
        contentType: 'application/json',
        body: [
          { progressMessage: 'Validating stack configuration.' },
          { progressMessage: 'Creating service visual-app.' },
          {
            progressMessage: 'Stack applied successfully.',
            severity: 'success',
            exitCode: 0,
            stackStatus: 'Healthy',
          },
        ]
          .map((item) => JSON.stringify(item))
          .join(''),
      });
    });

    await page.goto(`/stacks/edit/${fixture.stackId}`);
    await expect(page.getByText(fixture.name, { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Deploy', exact: true }).click();

    const confirmation = page.getByRole('dialog', { name: 'Confirm Deploy' });
    await confirmation.getByRole('textbox').fill(fixture.name);
    await confirmation.getByRole('button', { name: 'Deploy', exact: true }).click();

    const sheet = page.getByRole('dialog', { name: 'Stack' });
    await expect(sheet.getByText('Stack applied successfully.', { exact: true })).toBeVisible();
    await expect(page.locator('[data-sonner-toast]')).toBeHidden({ timeout: 10_000 });
    const elapsed = sheet.getByText(/ seconds$/).locator('..');
    await elapsed.evaluate((element) => {
      element.style.width = '6rem';
      element.style.flex = '0 0 6rem';
    });
    await locatorScreenshot(page, sheet, 'stack-task-sheet.png', [
      elapsed,
    ]);
  } finally {
    if (fixture) {
      if (!page.isClosed()) {
        await page.close();
      }
      await cleanupRuntimeFixture(request, fixture);
    }
  }
});

async function seedVisualTags(request: APIRequestContext) {
  await deleteTagsByPrefix(request, visualTagPrefix);
  const headers = { Authorization: `Bearer ${await getAdminAccessToken(request)}` };

  for (const tag of visualTags) {
    const response = await request.post('/api/v1/tags', {
      headers,
      data: tag,
    });
    if (!response.ok()) {
      throw new Error(`Unable to create visual tag '${tag.name}': HTTP ${response.status()}`);
    }
  }
}

const tagUpdatedAtValues = (page: Page) =>
  page.locator('tbody tr td:nth-child(5) span');

async function screenshot(page: Page, name: string, masks: Locator[] = []) {
  await page.evaluate(async () => {
    await document.fonts.ready;
  });

  await expect(page).toHaveScreenshot(name, {
    mask: [
      page.locator('[data-slot="sidebar-footer"] a[href*="github.com"]'),
      ...masks,
    ],
    maskColor: '#94A3B8',
  });
}

async function locatorScreenshot(
  page: Page,
  locator: Locator,
  name: string,
  masks: Locator[] = [],
) {
  await page.evaluate(async () => {
    await document.fonts.ready;
  });

  await expect(locator).toHaveScreenshot(name, {
    mask: masks,
    maskColor: '#94A3B8',
  });
}
