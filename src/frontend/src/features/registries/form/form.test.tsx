import { Routes, Route } from 'react-router';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { RegistryStatus, type RegistryConfigResponse } from '@/api/generated/api.types';
import { RegistryForm } from './form';

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
  configuration: { $type: 'DockerHub', userName: 'operator', hasPat: true },
};

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
