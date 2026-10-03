import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { NodeInspect } from './inspect';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value }: { value: string }) => <pre>{value}</pre>,
}));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('NodeInspect', () => {
  it('loads the live node inspection result', async () => {
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes/node-1/inspect`, () =>
        HttpResponse.json({ ...node, engineVersion: '29.1' }),
      ),
    );

    renderCitadel(<NodeInspect node={node} />);

    expect(await screen.findByText(/29.1/)).toBeVisible();
  });
});

const node = {
  id: 'node-1',
  versionIndex: 1,
  hostname: 'manager-1',
  name: 'manager-1',
  role: 'Manager',
  isLeader: true,
  reachability: 'Reachable',
  status: 'Ready',
  statusMessage: null,
  availability: 'Active',
  engineVersion: '29.0',
  operatingSystem: 'linux',
  architecture: 'x86_64',
  address: '10.0.0.1',
  labels: {},
  runningTaskCount: 1,
  desiredTaskCount: 1,
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T08:00:00Z',
  isStale: false,
  platformId,
  capabilities: null,
  tasks: [],
};
