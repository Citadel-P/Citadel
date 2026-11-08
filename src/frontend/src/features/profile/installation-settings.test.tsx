import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { MfaPolicy } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { createQueryClient } from '@/query-client-wrapper';
import { MfaPolicyControl as MfaSettingsControls } from '../instance/mfa-policy';
import { UpdateCheckSettings } from '../updates/config';
import { useRead } from '@/lib/hooks';

vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

// Separate settings surfaces share serialized mutations and one authoritative cache.
function InstallationSettingsControls() {
  return (
    <>
      <MfaSettingsControls />
      <UpdateCheckSettings />
    </>
  );
}

const url = 'http://localhost/api/v1/installation/settings';
const defaults = {
  updateChecksEnabled: true,
  updateChecksAllowed: true,
  updateCheckIntervalHours: 6,
  mfaPolicy: MfaPolicy.Optional,
  minimumMfaPolicy: MfaPolicy.Optional,
};

beforeEach(() => {
  server.use(
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json({ lastAttempt: null })),
    http.get('http://localhost/api/v1/updates/agents', () =>
      HttpResponse.json({
        targets: [],
        schedule: { includeCore: false, targets: [], enabled: false, cron: '0 3 * * *', timeZone: 'UTC' },
      }),
    ),
    http.get('http://localhost/api/v1/license/entitlements', () => HttpResponse.json({ capabilities: [] })),
  );
});

it('keeps MFA available during an update-check save and queues overlapping saves', async () => {
  let finishFirstSave!: () => void;
  const firstSave = new Promise<void>((resolve) => {
    finishFirstSave = resolve;
  });
  let settings = { ...defaults };
  const save = vi.fn();
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.patch(url, async ({ request }) => {
      const body = (await request.json()) as Partial<typeof settings>;
      save(body);
      if (body.updateChecksEnabled !== undefined) await firstSave;
      settings = { ...settings, ...body };
      return HttpResponse.json(settings);
    }),
  );
  const { user } = renderCitadel(<InstallationSettingsControls />);
  const updateSwitch = await screen.findByRole('switch', { name: 'Check automatically' });
  const policy = screen.getByRole('combobox', { name: 'Two-factor authentication policy' });
  try {
    await user.click(updateSwitch);
    expect(save).not.toHaveBeenCalled();
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(updateSwitch).toBeDisabled();
    expect(policy).toBeEnabled();
    await user.click(policy);
    await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
    expect(policy).toBeDisabled();
    expect(save).toHaveBeenCalledOnce();
  } finally {
    finishFirstSave();
  }
  await waitFor(() => expect(save).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(policy).toBeEnabled());
  expect(policy).toHaveTextContent('Required for administrators');
  expect(updateSwitch).not.toBeChecked();
  expect(updateSwitch).toBeEnabled();
});

it('saves installation choices separately and clears the cached update notice when disabled', async () => {
  let settings = { ...defaults };
  const save = vi.fn();
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.patch(url, async ({ request }) => {
      const body = (await request.json()) as Partial<typeof settings>;
      save(body);
      settings = { ...settings, ...body };
      return HttpResponse.json(settings);
    }),
  );
  const { user, queryClient } = renderCitadel(<InstallationSettingsControls />);
  queryClient.setQueryData(['getApplicationInfo', {}], { data: { availableUpdate: { version: '1.2.0' } } });
  const updateSwitch = await screen.findByRole('switch', { name: 'Check automatically' });
  expect(updateSwitch).toBeChecked();
  expect(screen.getByText(/MFA is optional/)).toBeVisible();
  await user.click(updateSwitch);
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(save).toHaveBeenCalledOnce());
  await waitFor(() =>
    expect(queryClient.getQueryData(['getApplicationInfo', {}])).toEqual({ data: { availableUpdate: undefined } }),
  );
  expect(updateSwitch).not.toBeChecked();
  expect(save).toHaveBeenCalledWith({ updateChecksEnabled: false });
  expect(queryClient.getQueryData(['getApplicationInfo', {}])).toEqual({ data: { availableUpdate: undefined } });
  await user.click(screen.getByRole('combobox', { name: 'Two-factor authentication policy' }));
  await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
  await screen.findByText(/Administrators using a local password/);
  expect(save).toHaveBeenLastCalledWith({ mfaPolicy: MfaPolicy.RequiredForAdministrators });
  expect(updateSwitch).not.toBeChecked();
});

it('shows server restrictions and excludes weaker MFA policies', async () => {
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        ...defaults,
        updateChecksAllowed: false,
        updateChecksEnabled: false,
        mfaPolicy: MfaPolicy.RequiredForAdministrators,
        minimumMfaPolicy: MfaPolicy.RequiredForAdministrators,
      }),
    ),
  );
  const { user } = renderCitadel(<InstallationSettingsControls />);
  const updateSwitch = await screen.findByRole('switch', { name: 'Check automatically' });
  expect(updateSwitch).toBeDisabled();
  expect(updateSwitch).not.toBeChecked();
  expect(screen.getByText('Disabled by server configuration.')).toBeVisible();
  await user.click(screen.getByRole('combobox', { name: 'Two-factor authentication policy' }));
  expect(screen.queryByRole('option', { name: 'Optional' })).not.toBeInTheDocument();
  expect(screen.getByRole('option', { name: 'Required for everyone' })).toBeVisible();
});

it.each([false, true])('keeps update status current when saving %s without realtime', async (enabled) => {
  const settings = { ...defaults, updateChecksEnabled: !enabled };
  const status = {
    checksEnabled: !enabled,
    checksAllowed: true,
    available: { version: '0.1.2' },
    lastSuccess: '2026-11-01T12:00:00Z',
  };
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.get('http://localhost/api/v1/updates', () => HttpResponse.json(status)),
    http.patch(url, () => HttpResponse.json({ ...settings, updateChecksEnabled: enabled })),
  );
  const { user, queryClient } = renderCitadel(<UpdateCheckSettings />);
  await waitFor(() =>
    expect(queryClient.getQueryData<{ data: typeof status }>(['getInstanceUpdates', {}])?.data).toEqual(status),
  );
  await user.click(await screen.findByRole('switch', { name: 'Check automatically' }));
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() =>
    expect(queryClient.getQueryData<{ data: typeof status }>(['getInstanceUpdates', {}])?.data).toEqual({
      ...status,
      checksEnabled: enabled,
    }),
  );
});

it('cancels an older update-status read before applying the saved check setting', async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  server.use(
    http.get(url, () => HttpResponse.json(defaults)),
    http.patch(url, () => HttpResponse.json({ ...defaults, updateChecksEnabled: false })),
  );
  const { user, queryClient } = renderCitadel(<UpdateCheckSettings />);
  const stale = { data: { checksEnabled: true, checksAllowed: true } };
  queryClient.setQueryData(['getInstanceUpdates', {}], stale);
  const pending = queryClient
    .fetchQuery({
      queryKey: ['getInstanceUpdates', {}],
      queryFn: async () => {
        await gate;
        return stale;
      },
    })
    .catch(() => undefined);
  try {
    await user.click(await screen.findByRole('switch', { name: 'Check automatically' }));
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    await waitFor(() =>
      expect(queryClient.getQueryData(['getInstanceUpdates', {}])).toMatchObject({
        data: { checksEnabled: false },
      }),
    );
    await act(async () => {
      release();
      await pending;
    });
    expect(queryClient.getQueryData(['getInstanceUpdates', {}])).toMatchObject({
      data: { checksEnabled: false },
    });
  } finally {
    release();
    await pending;
  }
});

it('restarts a pending initial status read when there is no cached status to patch', async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const read = vi.fn();
  server.use(
    http.get(url, () => HttpResponse.json(defaults)),
    http.patch(url, () => HttpResponse.json({ ...defaults, updateChecksEnabled: false })),
    http.get('http://localhost/api/v1/updates', async () => {
      read();
      if (read.mock.calls.length === 1) {
        await gate;
        return HttpResponse.json({ checksEnabled: true });
      }
      return HttpResponse.json({ checksEnabled: false });
    }),
  );
  function Status() {
    const status = useRead('getInstanceUpdates');
    return <p>{status.data ? `Checks enabled: ${status.data.data.checksEnabled}` : 'Loading status'}</p>;
  }
  const { user } = renderCitadel(
    <>
      <UpdateCheckSettings />
      <Status />
    </>,
  );
  try {
    await waitFor(() => expect(read).toHaveBeenCalledOnce());
    await user.click(await screen.findByRole('switch', { name: 'Check automatically' }));
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    expect(await screen.findByText('Checks enabled: false')).toBeVisible();
    expect(read).toHaveBeenCalledTimes(2);
  } finally {
    release();
  }
});

it('keeps the draft and saved choice separate when an update fails', async () => {
  server.use(
    http.get(url, () => HttpResponse.json(defaults)),
    http.patch(url, () => new HttpResponse(null, { status: 500 })),
  );
  const { user } = renderCitadel(<InstallationSettingsControls />);
  const updateSwitch = await screen.findByRole('switch', { name: 'Check automatically' });
  await user.click(updateSwitch);
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  expect(
    (await screen.findByText(/Update-check setting could not be saved/)).closest('[role="alert"]'),
  ).toBeInTheDocument();
  expect(updateSwitch).not.toBeChecked();
  expect(updateSwitch).toBeEnabled();
  await user.click(screen.getAllByRole('button', { name: 'Reset' })[0]);
  expect(updateSwitch).toBeChecked();
});

it('does not let an older settings read overwrite a successful save', async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const read = vi.fn();
  let saved = { ...defaults };
  server.use(
    http.get(url, async () => {
      read();
      await gate;
      return HttpResponse.json(defaults);
    }),
    http.patch(url, async ({ request }) => {
      saved = { ...saved, ...((await request.json()) as Partial<typeof defaults>) };
      return HttpResponse.json(saved);
    }),
  );
  const queryClient = createQueryClient();
  queryClient.setQueryData(['getInstallationSettings', {}], { data: defaults });
  const { user } = renderCitadel(<InstallationSettingsControls />, { queryClient });
  try {
    await waitFor(() => expect(read).toHaveBeenCalledOnce());
    const toggle = screen.getByRole('switch', { name: 'Check automatically' });
    await user.click(toggle);
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    await waitFor(() =>
      expect(queryClient.getQueryData(['getInstallationSettings', {}])).toMatchObject({
        data: { updateChecksEnabled: false },
      }),
    );
    await act(async () => {
      release();
    });
    await waitFor(() => expect(queryClient.isFetching()).toBe(0));
    expect(saved.updateChecksEnabled).toBe(false);
    expect(toggle).not.toBeChecked();
    expect(queryClient.getQueryData(['getInstallationSettings', {}])).toMatchObject({ data: saved });
  } finally {
    release();
  }
});

it('identifies the failed setting when another setting saves successfully', async () => {
  let rejectUpdateChecks = true;
  let saved = { ...defaults };
  server.use(
    http.get(url, () => HttpResponse.json(saved)),
    http.patch(url, async ({ request }) => {
      const body = (await request.json()) as Partial<typeof defaults>;
      if (body.updateChecksEnabled !== undefined && rejectUpdateChecks) return new HttpResponse(null, { status: 500 });
      saved = { ...saved, ...body };
      return HttpResponse.json(saved);
    }),
  );
  const { user } = renderCitadel(<InstallationSettingsControls />);
  const toggle = await screen.findByRole('switch', { name: 'Check automatically' });
  await user.click(toggle);
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  expect(
    (await screen.findByText(/Update-check setting could not be saved/)).closest('[role="alert"]'),
  ).toBeInTheDocument();
  await user.click(screen.getByRole('combobox', { name: 'Two-factor authentication policy' }));
  await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
  await screen.findByText(/Administrators using a local password/);
  expect(screen.getByText(/Update-check setting could not be saved/)).toBeVisible();
  expect(screen.queryByText('MFA policy could not be saved. Try again.')).not.toBeInTheDocument();
  rejectUpdateChecks = false;
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(screen.queryByText(/Update-check setting could not be saved/)).not.toBeInTheDocument());
  expect(toggle).not.toBeChecked();
  expect(screen.queryByText(/Update-check setting could not be saved/)).not.toBeInTheDocument();
});
