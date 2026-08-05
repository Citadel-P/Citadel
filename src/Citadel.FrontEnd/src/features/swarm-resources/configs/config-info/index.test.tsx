import { ResourceInfoView } from '@/pages/resource-info';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { ConfigInfoComponents } from '.';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('ConfigInfoComponents', () => {
  it('uses the standard metadata and labels sections without a status indicator', async () => {
    const fake = new FakeHubConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/configs/config-1`, () =>
        HttpResponse.json(config),
      ),
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
    );

    const { container } = renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/configs/:resourceId"
          element={<ResourceInfoView Components={ConfigInfoComponents} type="Config" />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/configs/config-1`,
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByText('app-config')).toBeVisible();
    expect(screen.getByText('Details')).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Templating driver' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Version' })).toBeVisible();
    expect(screen.getByText('Used by services')).toBeVisible();
    expect(screen.getByText('web')).toBeVisible();
    expect(screen.getByText('Labels')).toBeVisible();
    expect(screen.getByText('com.example.environment')).toBeVisible();
    expect(screen.getByText('production')).toBeVisible();
    expect(ConfigInfoComponents.Header.Indicator).toBeUndefined();
    expect(container.querySelector('.h-2.w-2.rounded-full')).not.toBeInTheDocument();
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
};
