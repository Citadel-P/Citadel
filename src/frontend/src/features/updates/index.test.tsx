import { screen, waitFor, within } from '@testing-library/react';
import { getDefaultStore } from 'jotai';
import { useTaskSheet } from '@/lib/atoms';
import { pendingInstanceUpdateAtom } from '@/lib/instance-update-state';
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

vi.mock('@/components/custom/task-sheet', () => ({
  default: function TaskSheet() {
    const { state } = useTaskSheet('Updates');
    return state.open ? <div data-testid="activity-drawer" /> : null;
  },
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => undefined }));
vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

const status = {
  instanceId: '019a0000-0000-7000-8000-000000000001',
  installedVersion: '0.1.1',
  databaseVersion: '18.6',
  databaseAvailable: null,
  databaseManaged: false,
  databaseInstallation: 'External',
  databaseUpdateReason:
    'PostgreSQL is managed outside this Citadel Compose installation. Update it using its own deployment or provider.',
  channel: 'stable',
  checksAllowed: true,
  checksEnabled: true,
  lastAttempt: '2026-11-01T12:00:00Z',
  lastSuccess: '2026-11-01T12:00:00Z',
  checkError: null,
  available: { version: '0.1.2', releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v0.1.2' },
  installationUnavailableReason:
    'In-app Core updates are not configured. Complete the one-time update setup on the Docker host.',
};
const settings = {
  updateChecksEnabled: true,
  updateChecksAllowed: true,
  updateCheckIntervalHours: 6,
  mfaPolicy: 'Optional',
  minimumMfaPolicy: 'Optional',
};

beforeEach(() => {
  getDefaultStore().set(pendingInstanceUpdateAtom, null);
  localStorage.clear();
  vi.spyOn(toast, 'success').mockImplementation(() => 'notice');
  vi.spyOn(toast, 'error').mockImplementation(() => 'notice');
  server.use(
    http.get('http://localhost/api/v1/license/entitlements', () => HttpResponse.json({ capabilities: [] })),
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: true } })),
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json(status)),
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({ targets: [], schedule: { targets: [], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' } }),
    ),
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

it.each([
  ['Preparing', 'Preparing update'],
  ['Draining', 'Waiting for active work to finish'],
  ['Succeeded', 'Finishing update'],
])('hides release notices while an operation owns maintenance (%s)', async (phase, title) => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({
        ...status,
        operation: { id: 'update-operation', version: '0.1.2', phase, updatedAt: status.lastSuccess },
      }),
    ),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByText(title)).toBeVisible();
  expect(screen.queryByText(/Citadel 0.1.2 is available/)).not.toBeInTheDocument();
  expect(screen.getByText('An update is in progress or requires recovery.')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Update now' })).not.toBeInTheDocument();
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
  expect(screen.getByRole('link', { name: 'Set up in-app updates' })).toHaveAttribute(
    'href',
    'https://docs.citadelplane.com/operations/upgrade-and-rollback/#enable-in-app-updates',
  );
  expect(
    within(document.getElementById('available-updates')!).getByText(status.installationUnavailableReason, {
      exact: false,
    }),
  ).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Check now' }));
  await waitFor(() => expect(check).toHaveBeenCalledOnce());
  expect(await screen.findByText('0.1.3')).toBeVisible();
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
  expect(
    (await screen.findByText(/The official release check failed\./)).closest('[role="alert"]'),
  ).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Release notes' })).toBeVisible();
  await user.hover(screen.getByRole('button', { name: 'Update information' }));
  expect(await screen.findByRole('region', { name: 'Update information' })).toHaveTextContent(status.lastSuccess);
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
  expect(await screen.findByRole('switch', { name: 'Check automatically' })).toBeDisabled();
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
  expect(await screen.findByRole('switch', { name: 'Check automatically' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Add Updates' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('tab', { name: 'Activities' }));
  expect(screen.getByRole('tab', { name: 'Activities' })).toHaveAttribute('data-state', 'active');
  await user.click(screen.getByRole('tab', { name: 'Config' }));
  expect(screen.getByRole('tab', { name: 'Config' })).toHaveAttribute('data-state', 'active');
  expect(await screen.findByRole('switch', { name: 'Check automatically' })).toBeVisible();
});

it('reports a manual check failure while retaining the last successful release', async () => {
  server.use(http.post('http://localhost/api/v1/updates/check', () => new HttpResponse(null, { status: 500 })));
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  await user.click(await screen.findByRole('button', { name: 'Check now' }));
  await waitFor(() =>
    expect(
      screen.getAllByRole('alert').some((alert) => alert.textContent?.includes('The check could not finish.')),
    ).toBe(true),
  );
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
  const frequency = await screen.findByRole('combobox', { name: 'Frequency' });
  expect(frequency).toHaveTextContent('Every 6 hours');
  await user.click(frequency);
  await user.click(screen.getByRole('option', { name: 'Every 12 hours' }));
  expect(patch).not.toHaveBeenCalled();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(patch).toHaveBeenCalledWith({ updateCheckIntervalHours: 12 }));
  await waitFor(() => expect(frequency).toBeEnabled());
  await user.click(screen.getByRole('switch', { name: 'Check automatically' }));
  expect(frequency).toBeDisabled();
  expect(frequency).toHaveTextContent('Every 12 hours');
  expect(screen.getByRole('button', { name: 'Check now' })).toBeEnabled();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(patch).toHaveBeenLastCalledWith({ updateChecksEnabled: false }));
  expect(document.getElementById('release-checks')).toHaveTextContent('Next automatic checkDisabled');
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
      description: 'Release check complete. Review instance and Agent status.',
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
  await waitFor(() =>
    expect(screen.getAllByRole('alert').some((alert) => alert.textContent?.includes('GitHub is unavailable.'))).toBe(
      true,
    ),
  );
  expect(toast.error).toHaveBeenCalledWith('Update check failed', { description: 'GitHub is unavailable.' });
  expect(toast.success).not.toHaveBeenCalled();
  expect(screen.getByRole('link', { name: 'Release notes' })).toBeVisible();
});

it('keeps instance details in the tab info card and supports hover, keyboard focus, and click', async () => {
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  const trigger = await screen.findByRole('button', { name: 'Update information' });
  expect(screen.queryByText('Citadel 0.1.1')).not.toBeInTheDocument();
  await user.hover(trigger);
  const card = await screen.findByRole('region', { name: 'Update information' });
  expect(card).toHaveTextContent('Citadel 0.1.1');
  expect(card).toHaveTextContent('stable');
  expect(card).toHaveTextContent(status.lastSuccess);
  await user.unhover(trigger);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Update information' })).not.toBeInTheDocument());
  trigger.focus();
  expect(await screen.findByRole('region', { name: 'Update information' })).toBeVisible();
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Update information' })).not.toBeInTheDocument());
  await user.click(trigger);
  expect(await screen.findByRole('region', { name: 'Update information' })).toBeVisible();
});

it('keeps current Agents visible and updates only the selected outdated Agent while Core is current', async () => {
  const targets = ['A', 'B', 'C'].map((name) => ({
    id: name,
    name: `Agent ${name}`,
    kind: 'Platform',
    nodes: 1,
    supported: true,
    installedVersion: name === 'C' ? '0.1.0' : '0.1.1',
    availableVersion: '0.1.1',
    status: name === 'C' ? 'Available' : 'Current',
  }));
  const prepare = vi.fn();
  const approve = vi.fn();
  const save = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json({ ...status, available: null })),
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({ targets, schedule: { targets: ['A'], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' } }),
    ),
    http.post('http://localhost/api/v1/updates/agents/review', async ({ request }) => {
      prepare(await request.json());
      return HttpResponse.json({ id: 'review', targets: [targets[2]], expiresAt: '2099-01-01T00:00:00Z' });
    }),
    http.post('http://localhost/api/v1/updates/agents/approve', async ({ request }) => {
      approve(await request.json());
      return HttpResponse.json({ id: 'operation', finished: false, targets: [] });
    }),
    http.get('http://localhost/api/v1/updates/agents/operations/operation', () =>
      HttpResponse.json({ id: 'operation', finished: false, targets: [] }),
    ),
    http.put('http://localhost/api/v1/updates/schedule', () => {
      save();
      return HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByText('Up to date')).toBeVisible();
  expect(await screen.findByText('1 update available')).toBeVisible();
  expect(screen.getByText('2 up to date')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Update information' }));
  const info = await screen.findByRole('region', { name: 'Update information' });
  expect(info).toHaveTextContent('Citadel CoreUp to date');
  expect(info).toHaveTextContent('DatabaseIn-app updates unavailable');
  expect(within(info).getByText('PostgreSQL 18.6')).toBeVisible();
  expect(info).toHaveTextContent('1 Agent needs attention');
  expect(within(info).queryByText('Agent C')).not.toBeInTheDocument();
  const expand = within(info).getByRole('button', { name: /Agents.*1 Agent needs attention/ });
  expect(expand).toHaveAttribute('aria-expanded', 'false');
  await user.click(expand);
  expect(expand).toHaveAttribute('aria-expanded', 'true');
  expect(within(info).getByText('Agent C').closest('li')).toHaveTextContent('0.1.0 → 0.1.1');
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Review and update' })).toBeEnabled();
  await user.click(screen.getByRole('combobox', { name: 'Agents to update now' }));
  expect(screen.getByRole('option', { name: /Agent A/ })).toHaveAttribute('aria-disabled', 'true');
  expect(screen.getByRole('option', { name: /Agent B/ })).toHaveAttribute('aria-disabled', 'true');
  await user.click(screen.getByRole('option', { name: /Agent C/ }));
  await user.keyboard('{Escape}');
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Review and update' }));
  await waitFor(() => expect(prepare).toHaveBeenCalledWith({ targets: ['C'] }));
  expect(await screen.findByRole('dialog')).toHaveTextContent('Agent C');
  await user.click(screen.getByRole('button', { name: 'Update now' }));
  await waitFor(() => expect(approve).toHaveBeenCalledWith({ reviewId: 'review', requestId: expect.any(String) }));
  expect(save).not.toHaveBeenCalled();
  expect(await screen.findByRole('dialog')).toHaveTextContent('Progress is saved by Citadel');
  expect(screen.queryByTestId('activity-drawer')).not.toBeInTheDocument();
});

it('uses one form with the three sections and visible disabled Community schedule controls', async () => {
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  await screen.findByRole('button', { name: 'Check now' });
  const menus = screen.getAllByRole('navigation', { name: 'Form sections' });
  expect(menus).toHaveLength(1);
  const menu = within(menus[0]);
  expect(menu.getAllByRole('link').map((link) => link.textContent)).toEqual([
    'Available updates',
    'Release checks',
    'Scheduled updates',
  ]);
  const schedule = menu.getByRole('link', { name: 'Scheduled updates' });
  await user.click(schedule);
  expect(schedule).toHaveAttribute('aria-current', 'location');
  expect(document.querySelector(schedule.getAttribute('href')!)).toHaveTextContent(
    'Team enables scheduled installation.',
  );
  expect(screen.getByRole('checkbox', { name: 'Update Citadel Core' })).toBeDisabled();
  expect(screen.getByRole('textbox', { name: 'Cron' })).toBeDisabled();
});

it('keeps failed Agent schedule edits without resaving successful Core settings', async () => {
  let saved = { ...settings };
  const corePatch = vi.fn();
  const scheduleSave = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/license/entitlements', () =>
      HttpResponse.json({
        capabilities: [{ capability: 'AutomatedOperations', enabled: true }],
      }),
    ),
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({
        targets: [
          {
            id: 'C',
            name: 'Agent C',
            kind: 'Platform',
            nodes: 1,
            supported: true,
            installedVersion: '0.1.1',
            availableVersion: '0.1.1',
            status: 'Current',
          },
        ],
        schedule: { targets: ['C'], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' },
      }),
    ),
    http.get('http://localhost/api/v1/installation/settings', () => HttpResponse.json(saved)),
    http.patch('http://localhost/api/v1/installation/settings', async ({ request }) => {
      const body = (await request.json()) as Partial<typeof settings>;
      corePatch(body);
      saved = { ...saved, ...body };
      return HttpResponse.json(saved);
    }),
    http.put('http://localhost/api/v1/updates/schedule', async ({ request }) => {
      scheduleSave(await request.json());
      return scheduleSave.mock.calls.length === 1
        ? HttpResponse.json(
            { status: 400, title: 'Invalid schedule', detail: 'Select a registered Agent.' },
            { status: 400 },
          )
        : HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  const frequency = await screen.findByRole('combobox', { name: 'Frequency' });
  await user.click(frequency);
  await user.click(screen.getByRole('option', { name: 'Every 12 hours' }));
  const enabled = await screen.findByRole('switch', { name: 'Scheduled updates enabled' });
  await waitFor(() => expect(enabled).toBeEnabled());
  await user.click(enabled);
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  expect(await screen.findByText('Select a registered Agent.')).toBeVisible();
  expect(corePatch).toHaveBeenCalledExactlyOnceWith({ updateCheckIntervalHours: 12 });
  expect(enabled).toBeChecked();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(scheduleSave).toHaveBeenCalledTimes(2));
  expect(scheduleSave).toHaveBeenLastCalledWith({
    includeCore: false,
    targets: ['C'],
    enabled: true,
    cron: '0 3 * * *',
    timeZone: 'UTC',
  });
  expect(corePatch).toHaveBeenCalledOnce();
});

it.each([
  ['Current', 'All up to date'],
  ['Available', '1 update available'],
  ['Offline', '1 offline'],
  ['Unknown', '1 version unknown'],
  ['ManualRequired', '1 manual update required'],
  ['Incompatible', '1 no compatible update'],
])('shows independent Agent eligibility while Core is current (%s)', async (eligibility, summary) => {
  server.use(
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json({ ...status, available: null })),
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({
        targets: [
          {
            id: 'agent',
            name: 'Agent',
            kind: 'Platform',
            nodes: 1,
            supported: true,
            installedVersion: '0.1.1',
            status: eligibility,
          },
        ],
        schedule: { includeCore: false, targets: [], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' },
      }),
    ),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByText(summary)).toBeVisible();
  const review = screen.getByRole('button', { name: 'Review and update' });
  if (eligibility === 'Available') expect(review).toBeEnabled();
  else expect(review).toBeDisabled();
  expect(screen.getByText('Up to date')).toBeVisible();
  expect(screen.getByText('0.1.1')).toBeVisible();
});

it.each([
  [true, false],
  [false, true],
  [true, true],
])(
  'saves the shared schedule with Core=%s and Agents=%s without starting updates',
  async (includeCore, includeAgent) => {
    const save = vi.fn();
    const approve = vi.fn();
    server.use(
      http.get('http://localhost/api/v1/license/entitlements', () =>
        HttpResponse.json({ capabilities: [{ capability: 'AutomatedOperations', enabled: true }] }),
      ),
      http.get('http://localhost/api/v1/updates', () =>
        HttpResponse.json({ ...status, installationUnavailableReason: null }),
      ),
      http.get('http://localhost/api/v1/updates/agents', () =>
        HttpResponse.json({
          targets: [
            {
              id: 'A',
              name: 'Agent A',
              status: 'Current',
              installedVersion: '0.1.1',
              supported: true,
              kind: 'Platform',
              nodes: 1,
            },
          ],
          schedule: { includeCore: false, enabled: false, targets: [], cron: '0 3 * * 0', timeZone: 'Europe/Paris' },
          nextRun: null,
        }),
      ),
      http.put('http://localhost/api/v1/updates/schedule', async ({ request }) => {
        save(await request.json());
        return HttpResponse.json({});
      }),
      http.post('http://localhost/api/v1/updates/approve', () => {
        approve();
        return HttpResponse.json({});
      }),
    );
    const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
    const enabled = await screen.findByRole('switch', { name: 'Scheduled updates enabled' });
    expect(screen.getByRole('textbox', { name: 'Cron' })).toBeDisabled();
    await user.click(enabled);
    if (includeCore) await user.click(screen.getByRole('checkbox', { name: 'Update Citadel Core' }));
    if (includeAgent) {
      await user.click(screen.getByRole('combobox', { name: 'Scheduled Agents' }));
      await user.click(screen.getByRole('option', { name: /Agent A/ }));
      await user.keyboard('{Escape}');
    }
    // Unsaved scope/timing must not be presented as an active schedule.
    expect(document.getElementById('scheduled-updates')).toHaveTextContent('Next runDisabled');
    expect(save).not.toHaveBeenCalled();
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    await waitFor(() =>
      expect(save).toHaveBeenCalledWith({
        includeCore,
        targets: includeAgent ? ['A'] : [],
        enabled: true,
        cron: '0 3 * * 0',
        timeZone: 'Europe/Paris',
      }),
    );
    expect(approve).not.toHaveBeenCalled();
  },
);

it('lets an expired Team disable the retained scope while keeping the saved fields visible and disabled', async () => {
  const save = vi.fn();
  const schedule = { includeCore: true, enabled: true, targets: ['A'], cron: '0 3 * * 0', timeZone: 'Europe/Paris' };
  server.use(
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({ targets: [], schedule, nextRun: '2026-11-08T02:00:00Z' }),
    ),
    http.put('http://localhost/api/v1/updates/schedule', async ({ request }) => {
      save(await request.json());
      return HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  await user.click(await screen.findByRole('switch', { name: 'Scheduled updates enabled' }));
  expect(screen.getByRole('textbox', { name: 'Cron' })).toBeDisabled();
  expect(document.getElementById('scheduled-updates')).toHaveTextContent('Paused');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(save).toHaveBeenCalledWith({ ...schedule, enabled: false }));
});

it('shows the live database version without claiming an engine update is available', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({ ...status, available: null, databaseVersion: '17.11' }),
    ),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByText('PostgreSQL 17.11')).toBeVisible();
  expect(screen.getByText('In-app updates unavailable')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Review and update' })).toBeDisabled();
});

it('keeps Core status usable when the database version cannot be read', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({ ...status, available: null, databaseVersion: null }),
    ),
  );
  const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
  expect(await screen.findByText('Version unavailable')).toBeVisible();
  expect(screen.getByText('Up to date')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Update information' }));
  const card = await screen.findByRole('region', { name: 'Update information' });
  expect(within(card).getByText('Version unavailable')).toBeVisible();
  expect(within(card).queryByText('In-app updates unavailable')).not.toBeInTheDocument();
});

it.each([false, true])(
  'shows Agent release failures only for a registered inventory (hasAgents=%s)',
  async (hasAgents) => {
    const checkError = 'The Agent release matching Core could not be verified.';
    const targets = hasAgents
      ? [
          {
            id: 'A',
            name: 'Agent A',
            kind: 'Platform',
            nodes: 1,
            supported: true,
            installedVersion: '0.1.0',
            availableVersion: null,
            status: 'Unknown',
          },
        ]
      : [];
    server.use(
      http.get('http://localhost/api/v1/updates/agents', () =>
        HttpResponse.json({
          targets,
          checkError,
          schedule: { targets: [], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' },
        }),
      ),
    );
    const { user } = renderCitadel(<UpdatesPage />, { route: '/updates' });
    if (hasAgents) {
      expect(await screen.findByText(checkError)).toBeVisible();
      expect(screen.getByText('Check failed')).toBeVisible();
    } else {
      expect(await screen.findByText('No Agents registered')).toBeVisible();
      expect(screen.queryByText('0 registered')).not.toBeInTheDocument();
      expect(screen.queryByText(checkError)).not.toBeInTheDocument();
      expect(screen.queryByText('Check failed')).not.toBeInTheDocument();
    }
    await user.click(screen.getByRole('button', { name: 'Update information' }));
    const info = await screen.findByRole('region', { name: 'Update information' });
    if (hasAgents) {
      expect(within(info).getByText('Check failed')).toBeVisible();
      await user.click(within(info).getByRole('button', { name: /Agents.*Check failed/ }));
      expect(within(info).getByText(checkError)).toBeVisible();
    } else {
      expect(within(info).getByText('No Agents registered')).toBeVisible();
      expect(within(info).queryByText(checkError)).not.toBeInTheDocument();
      expect(within(info).queryByText('Check failed')).not.toBeInTheDocument();
    }
    expect(within(info).getByText('In-app updates unavailable')).toBeVisible();
    expect(within(info).getByText(status.databaseUpdateReason)).toBeVisible();
  },
);

it.each([
  [
    'Bundled',
    'Register this Compose installation to enable updates.',
    'In-app updates not configured',
    'Register the installation',
  ],
  ['Bundled', 'This PostgreSQL major version requires a manual migration.', 'Update blocked', null],
  ['Unknown', 'Could not verify how PostgreSQL is deployed.', 'Status unavailable', 'Register the installation'],
])(
  'distinguishes %s database eligibility from externally managed databases',
  async (databaseInstallation, databaseUpdateReason, badge, installationUnavailableReason) => {
    server.use(
      http.get('http://localhost/api/v1/updates', () =>
        HttpResponse.json({
          ...status,
          available: null,
          databaseInstallation,
          databaseUpdateReason,
          installationUnavailableReason,
        }),
      ),
    );
    renderCitadel(<UpdatesPage />, { route: '/updates/config' });
    expect(await screen.findByText(databaseUpdateReason as string)).toBeVisible();
    expect(screen.getByText(badge as string)).toBeVisible();
    expect(screen.queryByText('In-app updates unavailable')).not.toBeInTheDocument();
  },
);

it('shows Back to top on the Updates page when the main container is scrolled', async () => {
  const { fireEvent } = await import('@testing-library/react');
  renderCitadel(
    <div id="main-scroll-container">
      <UpdatesPage />
    </div>,
    { route: '/updates' },
  );
  await screen.findByRole('button', { name: 'Check now' });
  const container = document.getElementById('main-scroll-container')!;
  container.scrollTo = vi.fn();
  fireEvent.scroll(container, { target: { scrollTop: 500 } });
  fireEvent.click(await screen.findByRole('button', { name: 'Back to top' }));
  expect(container.scrollTo).toHaveBeenCalledWith(expect.objectContaining({ top: 0 }));
});

it('offers one-time update setup even when Core has no newer release', async () => {
  server.use(
    http.get('http://localhost/api/v1/updates', () =>
      HttpResponse.json({
        ...status,
        available: null,
        databaseInstallation: 'Bundled',
        databaseUpdateReason:
          'In-app updates are not configured. This does not mean a PostgreSQL upgrade is available.',
      }),
    ),
  );
  renderCitadel(<UpdatesPage />, { route: '/updates/config' });
  expect(await screen.findByText('In-app updates not configured')).toBeVisible();
  expect(screen.getAllByRole('link', { name: 'Set up in-app updates' })).toHaveLength(2);
  expect(screen.queryByText('Update available')).not.toBeInTheDocument();
  expect(screen.queryByText('Setup required')).not.toBeInTheDocument();
});
