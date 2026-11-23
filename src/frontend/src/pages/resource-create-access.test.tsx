import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { createQueryClient } from '@/query-client-wrapper';
import { ResourceForm } from './resource-form';

const form = vi.hoisted(() => vi.fn(() => <input aria-label="Resource name" />));
vi.mock('@/features/backup-policies/form', () => ({ BackupPolicyFormComponents: { AddForm: { Content: form } } }));
vi.mock('@/features/docker-resources/volumes', () => ({ VolumeFormComponents: { AddForm: { Content: form } } }));
vi.mock('@/features/oidc-providers/form', () => ({ OidcProviderFormComponents: { AddForm: { Content: form } } }));
vi.mock('@/features/access/users/form', () => ({ UserFormComponents: { AddForm: { Content: form } } }));

const api = 'http://localhost/api/v1';
function openAdd(route: string, queryClient = createQueryClient()) {
  return renderCitadel(
    <Routes>
      <Route path="/" element={<h1>Citadel home</h1>} />
      <Route path="/:type/add" element={<ResourceForm mode="add" />} />
      <Route path="/:type/:tab/add" element={<ResourceForm mode="add" />} />
      <Route path="/platforms/:platformId/:type/add" element={<ResourceForm mode="add" />} />
    </Routes>,
    { route, queryClient },
  );
}

beforeAll(async () => {
  await import('./resource-form-view');
});
beforeEach(() => {
  form.mockClear();
});

const profile = (writableResourceTypes: string[], isAdministrator = false) => ({
  authorization: { writableResourceTypes, isAdministrator },
});
const serveProfile = (types: string[], admin = false) =>
  server.use(http.get(api + '/profile', () => HttpResponse.json(profile(types, admin))));

it('rechecks a cached creation grant before loading the form and skips the collection read', async () => {
  const client = createQueryClient();
  client.setQueryData(['getCurrentProfile', {}], { data: profile(['BackupPolicy']) });
  const list = vi.fn(() => HttpResponse.json({ policies: [{ capabilities: { canWrite: true } }] }));
  server.use(http.get(api + '/backupPolicies', list));
  serveProfile(['BackupRepository']);
  openAdd('/backup-policies/add', client);
  expect(await screen.findByRole('heading', { name: 'Access denied' })).toBeVisible();
  expect(screen.getByRole('link', { name: 'Back to list' })).toHaveAttribute('href', '/backup-policies');
  expect(list).not.toHaveBeenCalled();
  expect(form).not.toHaveBeenCalled();
});

it('supports retrying a failed permission check and opens the form after the server grants Write', async () => {
  server.use(http.get(api + '/profile', () => new HttpResponse(null, { status: 503 })));
  const { user } = openAdd('/backup-policies/add');
  const retry = await screen.findByRole('button', { name: 'Retry' });
  expect(form).not.toHaveBeenCalled();
  serveProfile(['BackupPolicy']);
  await user.click(retry);
  expect(await screen.findByRole('textbox', { name: 'Resource name' })).toBeVisible();
});

it.each([false, true])('uses Write on the platform in the URL for volumes (canWrite=%s)', async (canWrite) => {
  server.use(
    http.get(api + '/platforms/selected', () => HttpResponse.json({ id: 'selected', capabilities: { canWrite } })),
  );
  openAdd('/platforms/selected/volumes/add');
  if (canWrite) expect(await screen.findByRole('textbox')).toBeVisible();
  else {
    expect(await screen.findByRole('link', { name: 'Back to list' })).toHaveAttribute(
      'href',
      '/platforms/selected/volumes',
    );
    expect(form).not.toHaveBeenCalled();
  }
});

it('requires administrator access for direct user creation', async () => {
  serveProfile(['User']);
  openAdd('/access/users/add');
  expect(await screen.findByRole('heading', { name: 'Access denied' })).toBeVisible();
  expect(form).not.toHaveBeenCalled();
});

it.each([403, 404])('preserves the distinction between forbidden and missing platforms (%s)', async (status) => {
  server.use(http.get(api + '/platforms/selected', () => new HttpResponse(null, { status })));
  const { user } = openAdd('/platforms/selected/volumes/add');
  expect(await screen.findByRole('heading', { name: status === 403 ? 'Access denied' : 'Oops!' })).toBeVisible();
  expect(form).not.toHaveBeenCalled();
  expect(screen.getByRole('img', { name: `Error ${status}` })).toBeVisible();
  await user.click(screen.getByRole('link', { name: 'Go to homepage' }));
  expect(await screen.findByRole('heading', { name: 'Citadel home' })).toBeVisible();
});

it('checks the rule permission independently of the notification channel permission', async () => {
  serveProfile(['AlertChannel']);
  openAdd('/alert-rules/add');
  expect(await screen.findByRole('heading', { name: 'Access denied' })).toBeVisible();
});

it('uses the managed resource grant for a platform-scoped service URL', async () => {
  serveProfile(['Platform']);
  openAdd('/platforms/selected/services/add');
  expect(await screen.findByRole('heading', { name: 'Access denied' })).toBeVisible();
});

it.each([
  { route: '/access/users/add', writes: ['User'], allowed: true },
  { route: '/access/users/add', writes: [], allowed: false },
  { route: '/oidc-providers/add', writes: [], allowed: true },
])('honors the administrator create requirement for $route with grants $writes', async ({ route, writes, allowed }) => {
  serveProfile(writes, true);
  openAdd(route);
  if (allowed) expect(await screen.findByRole('textbox')).toBeVisible();
  else expect(await screen.findByRole('heading', { name: 'Access denied' })).toBeVisible();
});
