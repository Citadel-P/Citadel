import { ResourceInfoView } from '@/pages/resource-info';
import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { ConfigInfoComponents } from '.';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('ConfigInfoComponents', () => {
  it('uses one Inspect tab with Details, referencing Services, and Labels sections', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/configs/config-1`, () =>
        HttpResponse.json(config),
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
            path="/platforms/:platformId/configs/:resourceId"
            element={<ResourceInfoView Components={ConfigInfoComponents} type="Config" />}
          />
        </Routes>
      </AppContext.Provider>,
      {
        route: `/platforms/${platformId}/configs/config-1`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByText('app-config')).toBeVisible();
    expect(screen.getByText('Details')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Templating driver' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Version' })).toBeVisible();
    expect(await screen.findByRole('link', { name: 'web' })).toBeVisible();
    expect(screen.getByText('Services using this config')).toBeVisible();
    expect(screen.getByText('Labels')).toBeVisible();
    expect(screen.getByText('com.example.environment')).toBeVisible();
    expect(screen.getByText('production')).toBeVisible();
    expect(screen.getAllByRole('tab')).toHaveLength(1);
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeVisible();
    expect(ConfigInfoComponents.Header.Indicator).toBeDefined();
    expect(container.querySelector('.h-2.w-2.rounded-full')).toBeInTheDocument();
  });
});

const config = {
  id: 'config-1',
  versionIndex: 3,
  name: 'app-config',
  templatingDriver: 'golang',
  serviceNames: ['web'],
  labels: { 'com.example.environment': 'production' },
  createdAt: '2026-08-04T11:00:00Z',
  updatedAt: '2026-08-04T12:00:00Z',
  observedAt: '2026-08-04T12:00:00Z',
  isStale: false,
  inUse: true,
};

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
  ports: [],
  networkIds: [],
  secretIds: [],
  configIds: ['config-1'],
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
