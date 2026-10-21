import { Routes, Route, useParams } from 'react-router';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { RegistryStatus, type RegistryConfigResponse } from '@/api/generated/api.types';
import { RegistryForm } from './form';
import { RegistryFormComponents } from './index';
import { useRegistryConfig } from '../queries';

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: (props: { original: unknown; modified: unknown }) => (
    <pre data-testid="diff">{JSON.stringify(props)}</pre>
  ),
}));

const resource: RegistryConfigResponse = {
  id: 'registry-1',
  name: 'private-registry',
  registryHost: 'docker.io',
  status: RegistryStatus.Active,
  description: '',
  tags: [],
  capabilities: { canRead: true, canWrite: true, canExecute: true },
  configuration: { $type: 'DockerHub', userName: 'operator', hasPat: true },
};

function RegistryConfigPage() {
  const { id } = useParams();
  const { data } = useRegistryConfig(id);
  const Config = RegistryFormComponents.EditForm!.Tabs[0].Content;
  return data ? <Config resource={data.data} /> : null;
}

it.each<RegistryConfigResponse['configuration']>([
  { $type: 'DockerHub', userName: 'operator', hasPat: true },
  { $type: 'GitHub', nameSpace: 'example', ghcrAuthEnabled: true, hasPat: true },
  { $type: 'Custom', authEnabled: true, userName: 'operator', hasPassword: true },
])('disables configuration controls for a read-only user (%j)', async (configuration) => {
  server.use(
    http.get('http://localhost/api/v1/registries/registry-1/_cfg', () =>
      HttpResponse.json({
        ...resource,
        configuration,
        capabilities: { canRead: true, canWrite: false, canExecute: false },
      }),
    ),
  );
  const { container } = renderCitadel(
    <Routes>
      <Route path="/registries/edit/:id" element={<RegistryConfigPage />} />
    </Routes>,
    { route: '/registries/edit/registry-1' },
  );

  await screen.findByRole('combobox', { name: 'Status' });
  const controls = container.querySelectorAll('fieldset input, fieldset [role="combobox"], fieldset [role="switch"]');
  expect(controls.length).toBeGreaterThan(2);
  for (const control of controls) expect(control).toBeDisabled();
  for (const save of screen.getAllByRole('button', { name: 'Save' })) expect(save).toBeDisabled();
  expect(screen.getByRole('combobox', { name: 'Jump to section' })).toBeEnabled();
});

it('keeps configuration editing available when the API grants Write permission', async () => {
  server.use(http.get('http://localhost/api/v1/registries/registry-1/_cfg', () => HttpResponse.json(resource)));
  const { user } = renderCitadel(
    <Routes>
      <Route path="/registries/edit/:id" element={<RegistryConfigPage />} />
    </Routes>,
    { route: '/registries/edit/registry-1' },
  );

  const username = await screen.findByDisplayValue('operator');
  expect(username).toBeEnabled();
  expect(screen.getByRole('combobox', { name: 'Status' })).toBeEnabled();
  await user.type(username, '-updated');
  expect(screen.getAllByRole('button', { name: 'Save' })[0]).toBeEnabled();
});

it('saves public settings without requesting or resubmitting a stored PAT', async () => {
  let payload: unknown;
  server.use(
    http.patch('http://localhost/api/v1/registries/registry-1', async ({ request }) => {
      payload = await request.json();
      return HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(
    <Routes>
      <Route path="/registries/edit/:id" element={<RegistryForm mode="edit" resource={resource} />} />
    </Routes>,
    { route: '/registries/edit/registry-1' },
  );
  expect(screen.getByPlaceholderText('****** — leave unchanged to keep')).toHaveValue('');
  await user.type(screen.getByDisplayValue('operator'), '-updated');
  const save = screen.getAllByRole('button', { name: 'Save' })[0];
  expect(save).toBeEnabled();
  await user.click(save);
  await waitFor(() =>
    expect(payload).toMatchObject({ configuration: { $type: 'DockerHub', userName: 'operator-updated' } }),
  );
  expect(JSON.stringify(payload)).not.toMatch(/pat|hasPat|\*{6}/);
});

it('masks a replacement in the diff while sending the actual replacement only on save', async () => {
  let payload: unknown;
  server.use(
    http.patch('http://localhost/api/v1/registries/registry-1', async ({ request }) => {
      payload = await request.json();
      return HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(
    <Routes>
      <Route path="/registries/edit/:id" element={<RegistryForm mode="edit" resource={resource} />} />
    </Routes>,
    { route: '/registries/edit/registry-1' },
  );
  await user.type(screen.getByPlaceholderText('****** — leave unchanged to keep'), 'replacement-token');
  await user.click(screen.getAllByRole('button', { name: 'Preview Changes' })[0]);
  expect(screen.getByTestId('diff')).not.toHaveTextContent('replacement-token');
  expect(screen.getByTestId('diff')).toHaveTextContent('******');
  await user.keyboard('{Escape}');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(payload).toMatchObject({ configuration: { pat: 'replacement-token' } }));
});
