import { screen, waitFor } from '@testing-library/react';
import { toast } from 'sonner';
import { useHTTPErrorHandler } from '@/lib/hooks';
import { delay, http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { Routes, Route } from 'react-router';
import ResourcePage from '@/pages/resource';

function UpdatesPage() {
  useHTTPErrorHandler();
  return (
    <Routes>
      <Route path=":type/:tab?" element={<ResourcePage />} />
    </Routes>
  );
}

vi.mock('@/lib/context/realtime-context', () => ({ useRealtimeContext: () => ({ liveConnectionState: 'connected' }) }));

vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: undefined }) }));

vi.mock('@/components/custom/task-sheet', () => ({ default: () => <div data-testid="activity-drawer" /> }));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => undefined }));
vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

const status = {
  instanceId: '019a0000-0000-7000-8000-000000000001',
  installedVersion: '0.1.1',
  channel: 'stable',
  checksAllowed: true,
  checksEnabled: true,
  lastAttempt: '2026-11-01T12:00:00Z',
  lastSuccess: '2026-11-01T12:00:00Z',
  checkError: null,
  available: { version: '0.1.2', releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v0.1.2' },
  installationUnavailableReason: 'Automatic installation is not enabled in this build.',
};
const settings = {
  updateChecksEnabled: true,
  updateChecksAllowed: true,
  updateCheckIntervalHours: 6,
  mfaPolicy: 'Optional',
  minimumMfaPolicy: 'Optional',
};

beforeEach(() => {
  localStorage.clear();
  vi.spyOn(toast, 'success').mockImplementation(() => 'notice');
  vi.spyOn(toast, 'error').mockImplementation(() => 'notice');
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: true } })),
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json(status)),
    http.get('http://localhost/api/v1/installation/settings', () => HttpResponse.json(settings)),
  );
});

it('denies direct navigation without fetching administrator status', async () => {
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: false } })),
    http.get('http://localhost/api/v1/updates', () => {
      read();
      return HttpResponse.json(status);
    }),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByRole('alert', {}, { timeout: 5000 })).toHaveTextContent(
    'Administrator access is required.',
  );
  expect(read).not.toHaveBeenCalled();
});

it('has exactly Config and Activities tabs and checks without installing or refetching status', async () => {
  const check = vi.fn();
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/updates', () => {
      read();
      return HttpResponse.json(status);
    }),
    http.post('http://localhost/api/v1/updates/check', () => {
      check();
      return HttpResponse.json({ ...status, available: { ...status.available, version: '0.1.3' } });
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByRole('heading', { name: 'Updates' })).toBeVisible();
  expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toEqual(['Config', 'Activities']);
  expect(await screen.findByRole('link', { name: 'Release notes' })).toHaveAttribute(
    'href',
    status.available.releaseUrl,
  );
  expect(screen.queryByRole('link', { name: 'Manual upgrade guide' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Check now' }));
  await waitFor(() => expect(check).toHaveBeenCalledOnce());
  expect(await screen.findByText(/Citadel 0.1.3 is available/)).toBeVisible();
  expect(read).toHaveBeenCalledOnce();
  expect(toast.success).toHaveBeenCalledWith('Citadel 0.1.3 is available', expect.any(Object));
  expect(screen.queryByTestId('activity-drawer')).not.toBeInTheDocument(); // Layout owns the activity drawer.
});

it('retains release information and shows a failed check separately', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({ ...status, checkError: 'The official release check failed.' }),
    ),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByRole('alert')).toHaveTextContent('The official release check failed.');
  expect(screen.getByRole('link', { name: 'Release notes' })).toBeVisible();
  await user.hover(screen.getByRole('button', { name: 'Instance information' }));
  expect(await screen.findByRole('region', { name: 'Instance information' })).toHaveTextContent(status.lastSuccess);
});

it('honors server policy for manual checks and the saved check setting', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({ ...status, checksAllowed: false, checksEnabled: false }),
    ),
    http.get('http://localhost/api/v1/installation/settings', () =>
      HttpResponse.json({ ...settings, updateChecksAllowed: false, updateChecksEnabled: false }),
    ),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByRole('button', { name: 'Check now' })).toBeDisabled();
  expect(await screen.findByRole('switch', { name: 'Automatic checks' })).toBeDisabled();
});

it('isolates Updates history from global activity URL filters and paging', async () => {
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/activities', ({ request }) => {
      read(new URL(request.url).searchParams);
      return HttpResponse.json({ pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 25 } });
    }),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates/activities?resourceType=Stack&eventType=StackApplied&page=9' });
  await waitFor(() => expect(read).toHaveBeenCalled());
  const query: URLSearchParams = read.mock.lastCall![0];
  expect(query.get('ResourceType')).toBe('Updates');
  expect(query.get('ResourceId')).toBe(status.instanceId);
  expect(query.get('EventType')).toBeNull();
  expect(query.get('Page')).toBe('1');
});

it('uses generic tab routes and lets Config override the remembered Activities tab', async () => {
  localStorage.setItem('Updates-active-tab', JSON.stringify('Activities'));
  server.use(
    http.get('http://localhost/api/v1/activities', () =>
      HttpResponse.json({ pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 25 } }),
    ),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  expect(await screen.findByRole('tab', { name: 'Config' })).toHaveAttribute('data-state', 'active');
  expect(await screen.findByRole('switch', { name: 'Automatic checks' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Add Updates' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('tab', { name: 'Activities' }));
  expect(screen.getByRole('tab', { name: 'Activities' })).toHaveAttribute('data-state', 'active');
  await user.click(screen.getByRole('tab', { name: 'Config' }));
  expect(screen.getByRole('tab', { name: 'Config' })).toHaveAttribute('data-state', 'active');
  expect(await screen.findByRole('switch', { name: 'Automatic checks' })).toBeVisible();
});

it('reports a manual check failure while retaining the last successful release', async () => {
  server.use(http.post('http://localhost/api/v1/updates/check', () => new HttpResponse(null, { status: 500 })));
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  await user.click(await screen.findByRole('button', { name: 'Check now' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('The check could not finish.');
  expect(screen.getByRole('link', { name: 'Release notes' })).toHaveAttribute('href', status.available.releaseUrl);
  expect(screen.getByRole('button', { name: 'Check now' })).toBeEnabled();
  expect(toast.error).toHaveBeenCalledOnce();
  expect(toast.success).not.toHaveBeenCalled();
});

it('saves frequency separately, preserves it when disabled, and still allows manual checks', async () => {
  let saved = { ...settings };
  const patch = vi.fn();
  const check = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/installation/settings', () => HttpResponse.json(saved)),
    http.patch('http://localhost/api/v1/installation/settings', async ({ request }) => {
      const body = (await request.json()) as Partial<typeof settings>;
      patch(body);
      saved = { ...saved, ...body };
      return HttpResponse.json(saved);
    }),
    http.post('http://localhost/api/v1/updates/check', () => {
      check();
      return HttpResponse.json({ ...status, checksEnabled: false });
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  const frequency = await screen.findByRole('combobox', { name: 'Check frequency' });
  expect(frequency).toHaveTextContent('Every 6 hours');
  await user.click(frequency);
  await user.click(screen.getByRole('option', { name: 'Every 12 hours' }));
  expect(patch).not.toHaveBeenCalled();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(patch).toHaveBeenCalledWith({ updateCheckIntervalHours: 12 }));
  await waitFor(() => expect(frequency).toBeEnabled());
  await user.click(screen.getByRole('switch', { name: 'Automatic checks' }));
  expect(frequency).toBeDisabled();
  expect(frequency).toHaveTextContent('Every 12 hours');
  expect(screen.getByRole('button', { name: 'Check now' })).toBeEnabled();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(patch).toHaveBeenLastCalledWith({ updateChecksEnabled: false }));
  expect(await screen.findByText('Next automatic check: Disabled')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Check now' }));
  await waitFor(() => expect(check).toHaveBeenCalledOnce());
  expect(saved.updateCheckIntervalHours).toBe(12);
});

it('keeps restricted reads blocked until a failed profile check is retried successfully', async () => {
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/profile', () => new HttpResponse(null, { status: 500 })),
    http.get('http://localhost/api/v1/updates', () => {
      read();
      return HttpResponse.json(status);
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not verify administrator access.');
  expect(read).not.toHaveBeenCalled();
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: true } })),
  );
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  expect(await screen.findByRole('heading', { name: 'Updates' })).toBeVisible();
  expect(await screen.findByRole('link', { name: 'Release notes' })).toBeVisible();
  expect(read).toHaveBeenCalledOnce();
});

it('shows progress and confirms a completed check even when no newer release is found', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json({ ...status, available: null })),
    http.post('http://localhost/api/v1/updates/check', async () => {
      await delay(150);
      return HttpResponse.json({ ...status, available: null });
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  await user.click(await screen.findByRole('button', { name: 'Check now' }));
  expect(await screen.findByRole('button', { name: 'Checking…' })).toBeDisabled();
  await waitFor(() =>
    expect(toast.success).toHaveBeenCalledWith('Check complete', {
      description: 'No newer release found.',
    }),
  );
  expect(screen.getByRole('button', { name: 'Check now' })).toBeEnabled();
});

it('does not report success when the response contains a failed check and retained release', async () => {
  server.use(
    http.post('http://localhost/api/v1/updates/check', () =>
      HttpResponse.json({
        ...status,
        checkError: 'GitHub is unavailable.',
      }),
    ),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  await user.click(await screen.findByRole('button', { name: 'Check now' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('GitHub is unavailable.');
  expect(toast.error).toHaveBeenCalledWith('Update check failed', { description: 'GitHub is unavailable.' });
  expect(toast.success).not.toHaveBeenCalled();
  expect(screen.getByRole('link', { name: 'Release notes' })).toBeVisible();
});

it('keeps instance details in the tab info card and supports hover, keyboard focus, and click', async () => {
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  const trigger = await screen.findByRole('button', { name: 'Instance information' });
  expect(screen.queryByText('Citadel 0.1.1')).not.toBeInTheDocument();
  await user.hover(trigger);
  const card = await screen.findByRole('region', { name: 'Instance information' });
  expect(card).toHaveTextContent('Citadel 0.1.1');
  expect(card).toHaveTextContent('stable');
  expect(card).toHaveTextContent(status.lastSuccess);
  await user.unhover(trigger);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Instance information' })).not.toBeInTheDocument());
  trigger.focus();
  expect(await screen.findByRole('region', { name: 'Instance information' })).toBeVisible();
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Instance information' })).not.toBeInTheDocument());
  await user.click(trigger);
  expect(await screen.findByRole('region', { name: 'Instance information' })).toBeVisible();
});
