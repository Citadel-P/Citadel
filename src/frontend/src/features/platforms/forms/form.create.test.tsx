import { PlatformConnectorType, PlatformType } from '@/api/generated/api.types';
import { useHTTPErrorHandler } from '@/lib/hooks';
import { createPlatform } from '@/test/factories/resources';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { toast } from 'sonner';
import { PlatformForm } from './form';

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: () => null,
  MonacoEditor: ({ value }: { value: string }) => <pre data-testid="agent-install-command">{value}</pre>,
}));

it.each([PlatformType.Docker, PlatformType.DockerSwarm])(
  'creates a Local %s platform without requesting Agent setup',
  async (type) => {
    const errorToast = vi.spyOn(toast, 'error');
    const platform = createPlatform({ type, connectorType: PlatformConnectorType.Local });
    let created = false;
    const agentSetup = vi.fn(() => HttpResponse.error());
    server.use(
      http.get('http://localhost/api/v1/platforms', () => HttpResponse.json({ platforms: created ? [platform] : [] })),
      http.get('http://localhost/api/v1/tags', () => HttpResponse.json({ tags: [] })),
      http.get('http://localhost/api/v1/platforms/agent/setup', agentSetup),
      http.post('http://localhost/api/v1/platforms', () => {
        created = true;
        return HttpResponse.json(platform);
      }),
    );
    function App() {
      useHTTPErrorHandler();
      return (
        <Routes>
          <Route path="/platforms/add" element={<PlatformForm mode="add" />} />
          <Route path="/platforms" element={<p>Platforms list</p>} />
        </Routes>
      );
    }
    const { user, queryClient } = renderCitadel(<App />, { route: '/platforms/add' });
    await waitFor(() => expect(queryClient.getQueryState(['listPlatforms', {}])?.status).toBe('success'));
    await user.type(screen.getByRole('textbox', { name: /Name/ }), 'demo-platform');
    if (type === PlatformType.DockerSwarm) {
      await user.click(screen.getByText('Docker Standalone').closest('button')!);
      await user.click(screen.getByRole('option', { name: /Docker Swarm/ }));
    }
    // Use the real FormShell: resetting its edits after save must not switch
    // the connector when the refreshed list includes the new Local platform.
    await user.click(screen.getAllByRole('button', { name: /^Save$/ })[0]);
    await screen.findByText('Platforms list');
    expect(agentSetup).not.toHaveBeenCalled();
    expect(errorToast).not.toHaveBeenCalled();
  },
);

it('keeps the automatically generated Agent ID in the install command, restored draft and create request', async () => {
  const local = createPlatform({ connectorType: PlatformConnectorType.Local });
  const auth = { accessToken: `x.${btoa(JSON.stringify({ sub: 'agent-setup-user', iss: 'test' }))}.x` };
  let submitted: any;
  server.use(
    http.get('http://localhost/api/v1/platforms', () => HttpResponse.json({ platforms: [local] })),
    http.get('http://localhost/api/v1/tags', () => HttpResponse.json({ tags: [] })),
    http.get('http://localhost/api/v1/platforms/agent/setup', ({ request }) => {
      const agentId = new URL(request.url).searchParams.get('agentId');
      return HttpResponse.json({
        agentId,
        hubPublicKey: 'public',
        requiresTls: false,
        dockerRunCommand: `docker run -e CITADEL_AGENT_ID="${agentId}" agent`,
      });
    }),
    http.post('http://localhost/api/v1/platforms', async ({ request }) => {
      submitted = await request.json();
      return HttpResponse.json(createPlatform({ id: submitted.agentId, connectorType: PlatformConnectorType.Agent }));
    }),
  );
  const first = renderCitadel(<PlatformForm mode="add" />, { auth });
  await screen.findByRole('textbox', { name: 'Agent Address' });
  await first.user.type(screen.getByRole('textbox', { name: /Name/ }), 'direct-agent');
  await first.user.type(screen.getByRole('textbox', { name: 'Agent Address' }), 'http://localhost:9001');
  const command = await screen.findByTestId('agent-install-command');
  await waitFor(() => expect(command.textContent).toMatch(/CITADEL_AGENT_ID="[0-9a-f-]{36}"/));
  const originalCommand = command.textContent!;
  const agentId = originalCommand.match(/CITADEL_AGENT_ID="([^"]+)"/)![1];
  expect(agentId).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  first.unmount();
  const second = renderCitadel(<PlatformForm mode="add" />, { auth });
  await waitFor(() => expect(screen.getByRole('textbox', { name: /Name/ })).toHaveValue('direct-agent'));
  await waitFor(() => expect(screen.getByTestId('agent-install-command').textContent).toBe(originalCommand));
  await second.user.click(screen.getAllByRole('button', { name: /^Save$/ })[0]);
  await waitFor(() => expect(submitted?.agentId).toBe(agentId));
  expect(submitted.address).toBe('http://localhost:9001');
});
