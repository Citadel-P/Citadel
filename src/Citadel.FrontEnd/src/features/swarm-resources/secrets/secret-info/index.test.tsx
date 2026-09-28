import { ResourceInfoView } from '@/pages/resource-info';
import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { SecretInfoComponents } from '.';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('SecretInfoComponents', () => {
  it('uses one Inspect tab without exposing the secret value', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/secrets/secret-1`, () =>
        HttpResponse.json(secret),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/services`, () =>
        HttpResponse.json({ items: [service] }),
      ),
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
    );

    const { container } = renderCitadel(
      <AppContext.Provider value={appContext}>
        <Routes>
          <Route
            path="/platforms/:platformId/secrets/:resourceId"
            element={<ResourceInfoView Components={SecretInfoComponents} type="Secret" />}
          />
        </Routes>
      </AppContext.Provider>,
      {
        route: `/platforms/${platformId}/secrets/secret-1`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByText('database-password')).toBeVisible();
    expect(screen.getByText('Details')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Driver' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Version' })).toBeVisible();
    expect(await screen.findByRole('link', { name: 'api' })).toBeVisible();
    expect(screen.getByText('Services using this secret')).toBeVisible();
    expect(screen.getByText('Labels')).toBeVisible();
    expect(screen.getByText('com.example.owner')).toBeVisible();
    expect(screen.getByText('platform')).toBeVisible();
    expect(screen.queryByText('actual-secret-value')).not.toBeInTheDocument();
    expect(screen.getAllByRole('tab')).toHaveLength(1);
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeVisible();
    expect(SecretInfoComponents.Header.Indicator).toBeDefined();
    expect(container.querySelector('.h-2.w-2.rounded-full')).toBeInTheDocument();
  });
});

const secret = {
  id: 'secret-1',
  versionIndex: 2,
  name: 'database-password',
  driver: null,
  serviceNames: ['api'],
  labels: { 'com.example.owner': 'platform' },
  createdAt: '2026-08-04T11:00:00Z',
  updatedAt: '2026-08-04T12:00:00Z',
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
  inUse: true,
};

const service = {
  id: 'service-1',
  versionIndex: 1,
  name: 'api',
  mode: 'Replicated',
  image: 'api:latest',
  runningTaskCount: 1,
  desiredTaskCount: 1,
  updateState: 'Completed',
  updateMessage: null,
  ports: [],
  networkIds: [],
  secretIds: ['secret-1'],
  configIds: [],
  labels: {},
  ownership: 'Unmanaged',
  dockerStackNamespace: null,
  ownershipDiagnostic: null,
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
};

const appContext = {
  isLoading: false,
  currentPlatform: {
    id: platformId,
    type: PlatformType.DockerSwarm,
    capabilities: {
      canRead: true,
      canWrite: true,
      canExecute: true,
      canViewLogs: true,
      canInspect: true,
      canOpenTerminal: false,
      canPull: false,
    },
  } as PlatformView,
  platforms: [],
  applicationInfo: undefined,
  unresolvedAlertCount: 0,
  liveAlertEvents: {},
  receivedAlertEventIds: [],
};
