import { SwarmServiceOwnership } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ServiceInspect } from './inspect';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value }: { value: string }) => <pre>{value}</pre>,
}));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('ServiceInspect', () => {
  it('loads the live service inspection result', async () => {
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services/service-1/inspect`, () =>
        HttpResponse.json({ ...service, image: 'nginx:inspected' }),
      ),
    );

    renderCitadel(<ServiceInspect service={service} />);

    expect(await screen.findByText(/nginx:inspected/)).toBeVisible();
  });
});

const service = {
  id: 'service-1',
  versionIndex: 1,
  name: 'web',
  mode: 'Replicated',
  image: 'nginx:latest',
  runningTaskCount: 1,
  desiredTaskCount: 1,
  updateState: 'Completed',
  updateMessage: null,
  ports: ['80/tcp'],
  networkIds: [],
  secretIds: [],
  configIds: [],
  labels: {},
  ownership: SwarmServiceOwnership.Unmanaged,
  dockerStackNamespace: null,
  ownershipDiagnostic: null,
  createdAt: '2026-08-04T11:00:00Z',
  updatedAt: '2026-08-04T12:00:00Z',
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
  platformId,
  capabilities: undefined,
  tasks: [],
};
