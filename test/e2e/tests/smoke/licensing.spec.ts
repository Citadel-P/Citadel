import { getAdminAccessToken } from '../../support/admin-api';
import { test, expect } from '../../support/test';

test('custom access control is explained and enforced for Community', async ({ page, request }) => {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };
  const licenseResponse = await request.get('/api/v1/license/entitlements', {
    headers,
  });
  expect(licenseResponse.ok()).toBe(true);

  const license = (await licenseResponse.json()) as {
    status: string;
    effectiveEdition: string;
    capabilities: Array<{
      capability: string;
      enabled: boolean;
    }>;
  };
  const customAccessControl = license.capabilities.find(
    (capability) => capability.capability === 'CustomAccessControl',
  );

  expect(license.status).toBe('Community');
  expect(license.effectiveEdition).toBe('Community');
  expect(customAccessControl).toMatchObject({ enabled: false });

  await page.goto('/access/roles');
  const addRoleButton = page.getByRole('button', { name: 'Add Role' });
  await expect(addRoleButton).toBeDisabled();

  await page.getByTestId('custom-role-license').hover();
  const licenseTooltip = page.getByRole('tooltip');
  await expect(licenseTooltip).toContainText('Create custom roles and permission sets.');
  await expect(licenseTooltip.getByLabel('Requires a Team license')).toBeVisible();

  const createResponse = await request.post('/api/v1/roles', {
    headers,
    data: {
      name: `e2e-license-bypass-${Date.now()}`,
      permissions: [],
    },
  });
  expect(createResponse.status()).toBe(403);

  const problem = (await createResponse.json()) as {
    type?: string;
    detail?: string;
    capability?: string;
    licenseStatus?: string;
    effectiveEdition?: string;
  };
  expect(problem).toMatchObject({
    type: 'https://citadel.local/problems/license-capability-required',
    detail: 'Custom access control requires a Team license.',
    capability: 'CustomAccessControl',
    licenseStatus: 'Community',
    effectiveEdition: 'Community',
  });
});
