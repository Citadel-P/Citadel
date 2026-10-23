import { screen, waitFor } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BuildPoolFormComponents } from './form';
import type { AuthorizedPool } from '@/api/generated/api.types';

vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));

it('offers self-managed builders by default and does not offer AWS for new pools', async () => {
  const Content = BuildPoolFormComponents.AddForm.Content!;
  const { user } = renderCitadel(<Content />);
  const provider = screen.getByRole('combobox', { name: 'Type' });
  expect(provider).toHaveTextContent('Self-managed');
  await user.click(provider);
  expect(screen.queryByRole('option', { name: 'AWS EC2' })).not.toBeInTheDocument();
});

it('preserves saved unsupported provider data while disabling an existing pool', async () => {
  const providerSpec = {
    $type: 'AwsEc2',
    region: 'eu-west-1',
    instanceType: 'saved-instance',
    tags: { keep: 'unchanged' },
  };
  const resource = {
    id: 'pool-1',
    name: 'stored AWS',
    enabled: true,
    providerSpec,
    maxActiveBuilders: 1,
    queueTimeoutSeconds: 3600,
    provisioningTimeoutSeconds: 600,
    registrationTimeoutSeconds: 300,
    heartbeatTimeoutSeconds: 90,
    cleanupTimeoutSeconds: 600,
    maximumInstanceLifetimeSeconds: 7200,
    failureRetentionMinutes: 0,
    capabilities: { canRead: true, canWrite: true, canExecute: true },
  } as unknown as AuthorizedPool;
  let payload: any;
  server.use(
    http.patch('http://localhost/api/v1/buildAgentPools/pool-1', async ({ request }) => {
      payload = await request.json();
      return HttpResponse.json({});
    }),
  );
  const Content = BuildPoolFormComponents.EditForm!.Tabs[0].Content;
  const { user } = renderCitadel(
    <Routes>
      <Route path="/build-pools/edit/:id" element={<Content resource={resource as any} />} />
    </Routes>,
    { route: '/build-pools/edit/pool-1' },
  );
  expect(screen.getByRole('combobox', { name: 'Type' })).toBeDisabled();
  expect(screen.getByDisplayValue('eu-west-1')).toBeDisabled();
  await user.click(screen.getByRole('switch', { name: 'Enabled' }));
  const save = screen.getAllByRole('button', { name: 'Save' })[0];
  await waitFor(() => expect(save).toBeEnabled());
  await user.click(save);
  await waitFor(() => expect(payload?.enabled).toBe(false));
  expect(payload).not.toHaveProperty('providerSpec');
});

it('loads the install command for the saved inbound pool identity', async () => {
  const id = '7fb4c244-485a-4b80-9611-b29e456adf65';
  const setup = vi.fn(() =>
    HttpResponse.json({
      agentId: id,
      dockerRunCommand: `docker run -e CITADEL_AGENT_ID=${id} agent`,
    }),
  );
  server.use(http.get(`http://localhost/api/v1/buildAgentPools/${id}/agent/setup`, setup));
  const resource = {
    id,
    name: 'builder',
    enabled: true,
    providerSpec: { $type: 'SelfManagedVm', connectionMode: 'InboundAgent', endpoint: 'http://localhost:9001' },
    capabilities: { canRead: true, canWrite: true },
  } as unknown as AuthorizedPool;
  const Content = BuildPoolFormComponents.EditForm!.Tabs[0].Content;
  renderCitadel(<Content resource={resource as any} />);
  await waitFor(() => expect(setup).toHaveBeenCalledOnce());
  expect(screen.getByRole('heading', { name: 'Agent Setup' })).toBeInTheDocument();
  expect(screen.queryByRole('textbox', { name: /Agent ID/ })).not.toBeInTheDocument();
});
