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

vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));

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
