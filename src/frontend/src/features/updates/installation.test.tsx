import { screen, waitFor } from '@testing-library/react';
import { getDefaultStore } from 'jotai';
import { toast } from 'sonner';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { pendingInstanceUpdateAtom } from '@/lib/instance-update-state';
import { RealtimeContext } from '@/lib/context/realtime-context';
import { DatabaseInstallationView, type UpdateStatusView } from '@/api/generated/api.types';
import { UpdateProgress } from './installation';
import { ReviewUpdates } from './review';

const status: UpdateStatusView = {
  instanceId: '019a0000-0000-7000-8000-000000000001',
  installedVersion: '0.1.1',
  databaseVersion: '18.6',
  databaseAvailable: null,
  databaseManaged: false,
  databaseInstallation: DatabaseInstallationView.Unknown,
  databaseUpdateReason: null,
  channel: 'stable',
  checksAllowed: true,
  checksEnabled: true,
  lastAttempt: null,
  lastSuccess: null,
  checkError: null,
  available: { version: '0.1.2', releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v0.1.2' },
  installationUnavailableReason: null,
  operation: null,
};
const operation = {
  id: '019a0000-0000-7000-8000-000000000002',
  version: '0.1.2',
  phase: 'Preparing',
  updatedAt: '2026-11-03T12:00:00Z',
};
afterEach(() => vi.unstubAllGlobals());

beforeEach(() => {
  getDefaultStore().set(pendingInstanceUpdateAtom, null);
  vi.spyOn(toast, 'success').mockImplementation(() => 'notice');
  vi.spyOn(toast, 'error').mockImplementation(() => 'notice');
});

it.each([true, false])(
  'retains the approval key after an ambiguous failure (randomUUID available: %s)',
  async (secureOrigin) => {
    if (!secureOrigin) vi.stubGlobal('crypto', { getRandomValues: crypto.getRandomValues.bind(crypto) });
    const requests: unknown[] = [];
    server.use(
      http.post('http://localhost/api/v1/updates/review', async ({ request }) => {
        expect(await request.json()).toEqual({ version: '0.1.2' });
        return HttpResponse.json({
          id: operation.id,
          installedVersion: '0.1.1',
          targetVersion: '0.1.2',
          expiresAt: '2026-11-03T13:00:00Z',
        });
      }),
      http.post('http://localhost/api/v1/updates/approve', async ({ request }) => {
        requests.push(await request.json());
        return requests.length === 1 ? new HttpResponse(null, { status: 503 }) : HttpResponse.json(operation);
      }),
    );
    const { user } = renderCitadel(<ReviewUpdates status={status} />);
    await user.click(screen.getByRole('button', { name: 'Review and update' }));
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent('Installed0.1.1');
    expect(dialog).toHaveTextContent('New version0.1.2');
    expect(requests).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Update and restart' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Approval could not be confirmed');
    await user.click(screen.getByRole('button', { name: 'Update and restart' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(requests).toHaveLength(2);
    expect(requests[0]).toEqual(requests[1]);
    expect(requests[0]).toEqual(
      expect.objectContaining({
        requestId: expect.stringMatching(/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/),
      }),
    );
    expect(getDefaultStore().get(pendingInstanceUpdateAtom)?.id).toBe(operation.id);
  },
);

it('does not offer installation when registration is unavailable', () => {
  renderCitadel(<ReviewUpdates status={{ ...status, installationUnavailableReason: 'Not registered' }} />);
  expect(screen.getByRole('button', { name: 'Review and update' })).toBeDisabled();
});

it('recovers a retained operation from the API without inventing progress while disconnected', async () => {
  getDefaultStore().set(pendingInstanceUpdateAtom, { instanceId: status.instanceId, id: operation.id });
  const read = vi.fn();
  server.use(
    http.get(`http://localhost/api/v1/updates/operations/${operation.id}`, () => {
      read();
      return HttpResponse.json({ ...operation, phase: 'Succeeded' });
    }),
  );
  renderCitadel(
    <RealtimeContext.Provider value={{ liveConnectionState: 'connected', retryConnection: async () => {} }}>
      <UpdateProgress status={status} />
    </RealtimeContext.Provider>,
  );
  expect(await screen.findByText('Update completed')).toBeVisible();
  expect(read).toHaveBeenCalledOnce();
});

it('shows an offline wait instead of advancing the saved phase', () => {
  renderCitadel(
    <RealtimeContext.Provider value={{ liveConnectionState: 'disconnected', retryConnection: async () => {} }}>
      <UpdateProgress status={{ ...status, operation: { ...operation, phase: 'Draining' } }} />
    </RealtimeContext.Provider>,
  );
  expect(screen.getByText('Waiting for active work to finish')).toBeVisible();
  expect(screen.getByText(/Waiting for Citadel to reconnect/)).toBeVisible();
  expect(getDefaultStore().get(pendingInstanceUpdateAtom)?.restartUntil).toBeGreaterThan(Date.now());
});

const reload = vi.fn();
afterEach(() => vi.unstubAllGlobals());

beforeEach(() => {
  reload.mockReset();
  vi.stubGlobal('location', {
    href: 'http://localhost/updates/activities?filter=updates#operation',
    reload,
  });
});
afterEach(() => vi.unstubAllGlobals());

function progress(current: UpdateStatusView, connected = true) {
  return (
    <RealtimeContext.Provider
      value={{ liveConnectionState: connected ? 'connected' : 'disconnected', retryConnection: async () => {} }}>
      <UpdateProgress status={current} />
    </RealtimeContext.Provider>
  );
}

function retainedOutcome(phase = 'Succeeded') {
  getDefaultStore().set(pendingInstanceUpdateAtom, {
    instanceId: status.instanceId,
    id: operation.id,
    restartUntil: Date.now() + 60_000,
  });
  server.use(
    http.get(`http://localhost/api/v1/updates/operations/${operation.id}`, () =>
      HttpResponse.json({ ...operation, phase }),
    ),
  );
}

it('leaves progress empty when there is no active or remembered update', () => {
  renderCitadel(progress(status));
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Reload portal' })).not.toBeInTheDocument();
  expect(reload).not.toHaveBeenCalled();
});

it('waits for maintenance release, reloads once, then clears the completed banner on remount', async () => {
  retainedOutcome();
  const target = { ...status, installedVersion: operation.version };
  const view = renderCitadel(progress({ ...target, operation: { ...operation, phase: 'Succeeded' } }));
  expect(screen.getByText('Finishing update')).toBeVisible();
  expect(screen.queryByText('Update completed')).not.toBeInTheDocument();
  expect(getDefaultStore().get(pendingInstanceUpdateAtom)?.restartUntil).toBeGreaterThan(Date.now());
  expect(reload).not.toHaveBeenCalled();

  view.rerender(progress(target));
  await waitFor(() => expect(reload).toHaveBeenCalledOnce());
  expect(toast.success).not.toHaveBeenCalled();
  expect(location.href).toBe('http://localhost/updates/activities?filter=updates#operation');
  expect(JSON.parse(sessionStorage.getItem('citadel-instance-update')!)).toEqual({
    instanceId: status.instanceId,
    id: operation.id,
    frontendReloaded: true,
  });
  view.unmount();
  renderCitadel(progress(target), { queryClient: view.queryClient });
  await waitFor(() => expect(getDefaultStore().get(pendingInstanceUpdateAtom)).toBeNull());
  expect(screen.queryByText('Update completed')).not.toBeInTheDocument();
  expect(JSON.parse(sessionStorage.getItem('citadel-instance-update')!)).toBeNull();
  expect(reload).toHaveBeenCalledOnce();
  expect(toast.success).toHaveBeenCalledExactlyOnceWith('Citadel updated successfully', {
    id: `instance-update-${status.instanceId}-${operation.id}`,
    description: 'Version 0.1.2 is now running.',
  });
});

it('waits for reconnection before reloading a completed target', async () => {
  retainedOutcome();
  const target = { ...status, installedVersion: operation.version };
  const view = renderCitadel(progress(target, false));
  expect(await screen.findByText('Update completed')).toBeVisible();
  expect(reload).not.toHaveBeenCalled();
  view.rerender(progress(target));
  await waitFor(() => expect(reload).toHaveBeenCalledOnce());
});

it('clears a completed update retained by an already reloaded tab without another reload', async () => {
  retainedOutcome();
  getDefaultStore().set(pendingInstanceUpdateAtom, {
    instanceId: status.instanceId,
    id: operation.id,
    frontendReloaded: true,
  });
  renderCitadel(progress({ ...status, installedVersion: operation.version }));
  await waitFor(() => expect(getDefaultStore().get(pendingInstanceUpdateAtom)).toBeNull());
  expect(screen.queryByText('Update completed')).not.toBeInTheDocument();
  expect(reload).not.toHaveBeenCalled();
});

it('does not reload on a historical success for a different running version', async () => {
  retainedOutcome();
  renderCitadel(progress(status));
  expect(await screen.findByText('Update completed')).toBeVisible();
  expect(reload).not.toHaveBeenCalled();
  expect(toast.success).not.toHaveBeenCalled();
});

it.each(['Failed', 'RecoveryRequired'])(
  'notifies once for %s while retaining recovery state across remounts',
  async (phase) => {
    const current = { ...status, operation: { ...operation, phase } };
    const view = renderCitadel(progress(current));
    expect(toast.error).toHaveBeenCalledExactlyOnceWith('Citadel update failed', {
      id: `instance-update-${status.instanceId}-${operation.id}`,
      description: 'The update to 0.1.2 needs manual recovery. Check Activities for details.',
    });
    expect(getDefaultStore().get(pendingInstanceUpdateAtom)).toMatchObject({
      instanceId: status.instanceId,
      id: operation.id,
      failureNotified: true,
    });
    view.rerender(progress({ ...current, operation: { ...current.operation } }));
    view.unmount();
    renderCitadel(progress(current), { queryClient: view.queryClient });
    expect(screen.getByText('Target version: Citadel 0.1.2.')).toBeVisible();
    expect(toast.error).toHaveBeenCalledOnce();
    expect(toast.success).not.toHaveBeenCalled();
    expect(reload).not.toHaveBeenCalled();
  },
);

it('still reports a failure when session storage is unavailable', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('Storage unavailable', 'SecurityError');
  });
  renderCitadel(progress({ ...status, operation: { ...operation, phase: 'Failed' } }));
  expect(toast.error).toHaveBeenCalledOnce();
  expect(screen.getByText('Update failed — operator recovery required')).toBeVisible();
  expect(reload).not.toHaveBeenCalled();
});

it.each(['Failed', 'Postponed', 'RecoveryRequired'])('does not reload a %s operation', async (phase) => {
  retainedOutcome(phase);
  renderCitadel(progress({ ...status, installedVersion: operation.version }));
  expect(await screen.findByText(`Target version: Citadel ${operation.version}.`)).toBeVisible();
  expect(reload).not.toHaveBeenCalled();
  expect(screen.queryByRole('button', { name: 'Reload portal' })).not.toBeInTheDocument();
});

it('ignores a retained operation belonging to another instance', () => {
  retainedOutcome();
  renderCitadel(progress({ ...status, instanceId: 'another-instance', installedVersion: operation.version }));
  expect(reload).not.toHaveBeenCalled();
  expect(screen.queryByText('Update completed')).not.toBeInTheDocument();
  expect(toast.success).not.toHaveBeenCalled();
  expect(toast.error).not.toHaveBeenCalled();
});

it('leaves a manual reload action if a durable reload marker cannot be saved', async () => {
  retainedOutcome();
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('Storage unavailable', 'SecurityError');
  });
  const { user } = renderCitadel(progress({ ...status, installedVersion: operation.version }));
  await screen.findByText('Update completed');
  expect(reload).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Reload portal' }));
  expect(reload).toHaveBeenCalledOnce();
});

it('remembers an explicit reload when storage recovers and clears the banner on remount', async () => {
  retainedOutcome();
  const storage = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('Storage unavailable', 'SecurityError');
  });
  const target = { ...status, installedVersion: operation.version };
  const view = renderCitadel(progress(target));
  await screen.findByText('Update completed');
  expect(reload).not.toHaveBeenCalled();
  storage.mockRestore();
  await view.user.click(screen.getByRole('button', { name: 'Reload portal' }));
  expect(JSON.parse(sessionStorage.getItem('citadel-instance-update')!).frontendReloaded).toBe(true);
  expect(reload).toHaveBeenCalledOnce();
  view.unmount();
  renderCitadel(progress(target), { queryClient: view.queryClient });
  await waitFor(() => expect(getDefaultStore().get(pendingInstanceUpdateAtom)).toBeNull());
  expect(screen.queryByText('Update completed')).not.toBeInTheDocument();
  expect(reload).toHaveBeenCalledOnce();
});

it('reports database completion against the database version while Core remains unchanged', async () => {
  getDefaultStore().set(pendingInstanceUpdateAtom, {
    instanceId: status.instanceId,
    id: operation.id,
    frontendReloaded: true,
  });
  server.use(
    http.get(`http://localhost/api/v1/updates/operations/${operation.id}`, () =>
      HttpResponse.json({ ...operation, component: 'Database', version: '18.6', phase: 'Succeeded' }),
    ),
  );
  renderCitadel(progress({ ...status, databaseVersion: '18.6', installedVersion: '0.1.1' }));
  await waitFor(() =>
    expect(toast.success).toHaveBeenCalledWith(
      'PostgreSQL updated successfully',
      expect.objectContaining({ description: 'Version 18.6 is now running.' }),
    ),
  );
  expect(reload).not.toHaveBeenCalled();
  expect(getDefaultStore().get(pendingInstanceUpdateAtom)).toBeNull();
});
