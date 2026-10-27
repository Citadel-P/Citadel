import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { MfaPolicy } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { createQueryClient } from '@/query-client-wrapper';
import { InstallationSettingsControls } from './installation-settings';

const url = 'http://localhost/api/v1/installation/settings';
const defaults = {
  updateChecksEnabled: true,
  updateChecksAllowed: true,
  mfaPolicy: MfaPolicy.Optional,
  minimumMfaPolicy: MfaPolicy.Optional,
};

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
  const updateSwitch = await screen.findByRole('switch', { name: 'Check for Citadel updates' });
  const policy = screen.getByRole('combobox', { name: 'Two-factor authentication policy' });
  try {
    await user.click(updateSwitch);
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
  const updateSwitch = await screen.findByRole('switch', { name: 'Check for Citadel updates' });
  expect(updateSwitch).toBeChecked();
  expect(screen.getByText(/MFA is optional/)).toBeVisible();
  await user.click(updateSwitch);
  await waitFor(() => expect(updateSwitch).not.toBeChecked());
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
  const updateSwitch = await screen.findByRole('switch', { name: 'Check for Citadel updates' });
  expect(updateSwitch).toBeDisabled();
  expect(updateSwitch).not.toBeChecked();
  expect(screen.getByText('Disabled by server configuration.')).toBeVisible();
  await user.click(screen.getByRole('combobox', { name: 'Two-factor authentication policy' }));
  expect(screen.queryByRole('option', { name: 'Optional' })).not.toBeInTheDocument();
  expect(screen.getByRole('option', { name: 'Required for everyone' })).toBeVisible();
});

it('keeps the saved choice when an update fails', async () => {
  server.use(
    http.get(url, () => HttpResponse.json(defaults)),
    http.patch(url, () => new HttpResponse(null, { status: 500 })),
  );
  const { user } = renderCitadel(<InstallationSettingsControls />);
  const updateSwitch = await screen.findByRole('switch', { name: 'Check for Citadel updates' });
  await user.click(updateSwitch);
  expect(await screen.findByRole('alert')).toHaveTextContent('Update-check setting could not be saved.');
  expect(updateSwitch).toBeChecked();
  expect(updateSwitch).toBeEnabled();
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
    const toggle = screen.getByRole('switch', { name: 'Check for Citadel updates' });
    await user.click(toggle);
    await waitFor(() => expect(toggle).not.toBeChecked());
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
  const toggle = await screen.findByRole('switch', { name: 'Check for Citadel updates' });
  await user.click(toggle);
  expect(await screen.findByRole('alert')).toHaveTextContent('Update-check setting could not be saved.');
  await user.click(screen.getByRole('combobox', { name: 'Two-factor authentication policy' }));
  await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
  await screen.findByText(/Administrators using a local password/);
  expect(screen.getByRole('alert')).toHaveTextContent('Update-check setting could not be saved.');
  expect(screen.queryByText('MFA policy could not be saved. Try again.')).not.toBeInTheDocument();
  rejectUpdateChecks = false;
  await user.click(toggle);
  await waitFor(() => expect(toggle).not.toBeChecked());
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
