import { getAdminAccessToken } from '../../support/admin-api';
import { test, expect } from '../../support/test';

test('an exhausted custom-role quota is explained and enforced by the API', async ({ page, request }) => {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };
  const licenseResponse = await request.get('/api/v1/license', { headers });
  expect(licenseResponse.ok()).toBe(true);

  const license = (await licenseResponse.json()) as {
    status: string;
    limits: Array<{
      limit: string;
      current: number;
      maximum: number;
    }>;
  };
  const customRoleLimit = license.limits.find((limit) => limit.limit === 'CustomRoles');

  expect(license.status).toBe('Community');
  expect(customRoleLimit).toMatchObject({ current: 0, maximum: 0 });

  await page.goto('/access/roles');
  const addRoleButton = page.getByRole('button', { name: 'Add Role' });
  await expect(addRoleButton).toBeDisabled();

  await page.getByTestId('custom-role-quota').hover();
  await expect(page.getByRole('tooltip')).toContainText('Custom role quota reached (0 of 0)');

  const createResponse = await request.post('/api/v1/roles', {
    headers,
    data: {
      name: `e2e-quota-bypass-${Date.now()}`,
      permissions: [],
    },
  });
  expect(createResponse.status()).toBe(403);

  const problem = (await createResponse.json()) as {
    detail?: string;
    violations?: Array<{
      limit: string;
      current: number;
      requested: number;
      maximum: number;
    }>;
  };
  expect(problem.detail).toBe('License quota exceeded.');
  expect(problem.violations).toEqual([
    {
      limit: 'CustomRoles',
      current: 0,
      requested: 1,
      maximum: 0,
    },
  ]);
});
