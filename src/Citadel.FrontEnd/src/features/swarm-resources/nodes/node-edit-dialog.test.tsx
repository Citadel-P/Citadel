import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { SwarmNodeView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { NodeEditDialog } from './node-edit-dialog';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('NodeEditDialog', () => {
  it('updates labels without changing availability through the standard mutation endpoint', async () => {
    let requestBody: unknown;
    server.use(
      http.patch(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes/node-1`, async ({ request }) => {
        requestBody = await request.json();
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const { user } = renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/nodes/:nodeId"
          element={<NodeEditDialog resource={node} open onOpenChange={vi.fn()} />}
        />
      </Routes>,
      { route: `/platforms/${platformId}/nodes/node-1` },
    );

    expect(screen.queryByRole('button', { name: 'Active' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(requestBody).toEqual({
        versionIndex: 7,
        availability: 'Active',
        labels: { zone: 'west' },
      }),
    );
  });
});

const node: SwarmNodeView = {
  id: 'node-1',
  versionIndex: 7,
  hostname: 'worker-1',
  role: 'Worker',
  isLeader: false,
  reachability: '',
  status: 'Ready',
  statusMessage: null,
  availability: 'Active',
  engineVersion: '29.0',
  operatingSystem: 'linux',
  architecture: 'x86_64',
  address: '10.0.0.2',
  labels: { zone: 'west' },
  runningTaskCount: 1,
  desiredTaskCount: 1,
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-07T10:00:00Z',
  isStale: false,
  capabilities: {
    canRead: true,
    canWrite: true,
    canExecute: false,
    canViewLogs: false,
    canInspect: true,
    canOpenTerminal: false,
    canPull: false,
  },
};
