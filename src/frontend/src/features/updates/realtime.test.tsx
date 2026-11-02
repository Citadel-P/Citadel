import { act, screen, waitFor } from '@testing-library/react';
import { Routes, Route } from 'react-router';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import ResourcePage from '@/pages/resource';

vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: undefined }) }));
vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));
vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

it('updates the saved frequency through the existing installation realtime notification', async () => {
  let hours = 6;
  const lastAttempt = new Date().toISOString();
  const fake = new FakeRealtimeConnection();
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: true } })),
    http.get('http://localhost/api/v1/installation/settings', () =>
      HttpResponse.json({
        updateChecksEnabled: true,
        updateChecksAllowed: true,
        updateCheckIntervalHours: hours,
        mfaPolicy: 'Optional',
        minimumMfaPolicy: 'Optional',
      }),
    ),
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({
        installedVersion: '0.1.1',
        channel: 'stable',
        checksAllowed: true,
        checksEnabled: true,
        lastAttempt,
        lastSuccess: lastAttempt,
      }),
    ),
  );
  renderCitadel(
    <Routes>
      <Route path=":type/:tab" element={<ResourcePage />} />
    </Routes>,
    {
      route: '/updates/config',
      groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
    },
  );
  await screen.findByRole('heading', { name: 'Updates' }, { timeout: 5000 });
  expect(await screen.findByRole('combobox', { name: 'Check frequency' })).toHaveTextContent('Every 6 hours');
  hours = 12;
  act(() => fake.emit('InstallationSettingsChanged'));
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Check frequency' })).toHaveTextContent('Every 12 hours'),
  );
  const expectedCheck = new Date(Date.parse(lastAttempt) + 12 * 3_600_000).toISOString();
  const schedule = screen.getByText('Next automatic check:');
  expect(schedule).toHaveTextContent(expectedCheck);
  expect(schedule.querySelector('time')).toHaveAttribute('datetime', expectedCheck);
  expect(schedule.querySelector('time')).toHaveAttribute('title', expectedCheck);
});

it('receives scoped activity events without refreshing the history list', async () => {
  const instanceId = '019a0000-0000-7000-8000-000000000001';
  const historyRead = vi.fn();
  const fake = new FakeRealtimeConnection();
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: true } })),
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({
        instanceId,
        installedVersion: '0.1.1',
        channel: 'stable',
        checksAllowed: true,
        checksEnabled: true,
      }),
    ),
    http.get('http://localhost/api/v1/activities', () => {
      historyRead();
      return HttpResponse.json({ pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 25 } });
    }),
  );
  renderCitadel(
    <Routes>
      <Route path=":type/:tab" element={<ResourcePage />} />
    </Routes>,
    {
      route: '/updates/activities',
      groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
    },
  );
  await screen.findByRole('heading', { name: 'Updates' }, { timeout: 5000 });
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `activity:Updates:${instanceId}`));
  await waitFor(() => expect(historyRead).toHaveBeenCalledOnce());
  const activity = {
    id: '019a0000-0000-7000-8000-000000000002',
    resourceType: 'Updates',
    resourceId: instanceId,
    resourceName: 'Citadel',
    eventType: 'UpdateDiscovered',
    status: 'Success',
    createdAt: '2026-11-01T12:00:00Z',
    actorType: 'System',
    actorName: 'Citadel',
    info: { $type: 'UpdateDiscovered', version: '0.1.2', releaseUrl: 'https://example.com/release' },
  };
  act(() => {
    fake.emit('ActivityEventReceived', {
      ...activity,
      id: 'unrelated',
      resourceId: 'another-instance',
      info: { ...activity.info, version: '9.9.9' },
    });
    fake.emit('ActivityEventReceived', activity);
    fake.emit('ActivityEventReceived', activity);
  });
  expect(await screen.findByText('Citadel 0.1.2 is available')).toBeVisible();
  expect(screen.getAllByText('Citadel 0.1.2 is available')).toHaveLength(1);
  expect(screen.queryByText('Citadel 9.9.9 is available')).not.toBeInTheDocument();
  expect(historyRead).toHaveBeenCalledOnce();
});
