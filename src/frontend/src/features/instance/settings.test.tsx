import { screen, waitFor, act } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { LogFormatView, MfaPolicy, type InstallationSettingsView, type InstanceConfigurationView } from '@/api/generated/api.types';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { InstanceSettingsForm } from './settings';
import { InstanceComponents } from './index';
import { ResourceView } from '@/pages/resource-view';
import { instanceSections } from './fields';

const configuration: InstanceConfigurationView = {
  passwordMinimumLength: 15,
  browserSessionLifetimeDays: 30,
  serviceAccountDefaultTokenLifetimeDays: 90,
  serviceAccountMaximumTokenLifetimeDays: 365,
  serviceAccountMaximumActiveTokens: 10,
  accessTokenLifetimeMinutes: 15,
  statisticsCollectionIntervalSeconds: 10,
  statisticsWriteIntervalSeconds: 60,
  scriptsEnabled: true,
  scriptsMaximumParallelRuns: 4,
  scriptsMaximumRemoteRuns: 16,
  scriptsDefaultTimeoutSeconds: 300,
  scriptsMaximumTimeoutSeconds: 1800,
  buildPreparationSlots: 2,
  buildTrackingSlots: 16,
  buildCleanupEnabled: true,
  buildRetentionDays: 90,
  backupsEnabled: true,
  backupTrackingSlots: 16,
  backupDefaultTimeoutSeconds: 120,
  openapiEnabled: false,
  requireDirectAgentHttps: false,
  agentImageOverride: '',
  logVerbosity: 'info',
  logFormat: LogFormatView.Text,
};
const initial = (): InstallationSettingsView => ({
  configuration: { ...configuration },
  effectiveConfiguration: { ...configuration },
  runningConfiguration: { ...configuration },
  deploymentOverrides: {},
  restartRequired: false,
  mfaPolicy: MfaPolicy.Optional,
  minimumMfaPolicy: MfaPolicy.Optional,
  updateChecksEnabled: true,
  updateChecksAllowed: true,
  updateCheckIntervalHours: 6,
});
const url = 'http://localhost/api/v1/installation/settings';

beforeEach(() => {
  localStorage.clear();
});

it('registers an administrator page and exposes all 26 controls with server defaults and units', async () => {
  server.use(http.get(url, () => HttpResponse.json(initial())));
  renderCitadel(<InstanceSettingsForm />);
  expect(InstanceComponents.access).toBe('administrator');
  expect(instanceSections.map((s) => s.label)).toEqual([
    'Security',
    'Monitoring',
    'Scripts',
    'Builds',
    'Backups',
    'Advanced',
  ]);
  expect(instanceSections.flatMap((s) => s.fields)).toHaveLength(25);
  expect(await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' })).toHaveValue(15);
  expect(screen.getByRole('spinbutton', { name: 'Browser session lifetime (days)' })).toHaveValue(30);
  expect(screen.getByRole('combobox', { name: 'Two-factor authentication policy' })).toHaveTextContent('Optional');
  expect(screen.getByText(/Saving does not restart Core or Agents/)).toBeVisible();
});

it('saves only changed choices and distinguishes saved values from running values', async () => {
  let settings = initial();
  const requests: unknown[] = [];
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.patch(url, async ({ request }) => {
      const body = (await request.json()) as { configuration: Partial<InstanceConfigurationView> };
      requests.push(body);
      settings = {
        ...settings,
        configuration: { ...settings.configuration, ...body.configuration },
        effectiveConfiguration: { ...settings.effectiveConfiguration, ...body.configuration },
        restartRequired: true,
      };
      return HttpResponse.json(settings);
    }),
  );
  const { user } = renderCitadel(<InstanceSettingsForm />);
  const minimum = await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '20');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(requests).toEqual([{ configuration: { passwordMinimumLength: 20 } }]));
  expect(await screen.findByText(/Restart required: saved settings differ/)).toBeVisible();
  expect(screen.getByText('Saved choice: 20. Currently in use: 15.')).toBeVisible();
});

it('displays explicit deployment overrides and prevents conflicting edits', async () => {
  const settings = initial();
  settings.deploymentOverrides.passwordMinimumLength = 18;
  settings.effectiveConfiguration.passwordMinimumLength = 18;
  settings.runningConfiguration.passwordMinimumLength = 18;
  server.use(http.get(url, () => HttpResponse.json(settings)));
  renderCitadel(<InstanceSettingsForm />);
  const minimum = await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  expect(minimum).toHaveValue(18);
  expect(minimum).toBeDisabled();
  expect(screen.getByText('CITADEL_PASSWORDS_MINIMUM_LENGTH')).toBeVisible();
  expect(screen.getByText('Saved choice: 15. Currently in use: 18.')).toBeVisible();
});

it('validates ranges and coupled values before saving', async () => {
  const patch = vi.fn();
  server.use(
    http.get(url, () => HttpResponse.json(initial())),
    http.patch(url, () => {
      patch();
      return HttpResponse.json(initial());
    }),
  );
  const { user } = renderCitadel(<InstanceSettingsForm />);
  const minimum = await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '7');
  await user.tab();
  expect(screen.getByText('Enter a whole number from 8 to 128.')).toBeVisible();
  expect(screen.getAllByRole('button', { name: 'Save' })[0]).toBeDisabled();
  expect(patch).not.toHaveBeenCalled();
  await user.clear(minimum);
  await user.type(minimum, '15');
  const timeout = screen.getByRole('spinbutton', { name: 'Default script timeout (seconds)' });
  await user.clear(timeout);
  await user.type(timeout, '1801');
  await user.tab();
  expect(screen.getByText('Default timeout must not exceed the maximum.')).toBeVisible();
  expect(screen.getAllByRole('button', { name: 'Save' })[0]).toBeDisabled();
  expect(patch).not.toHaveBeenCalled();

});

it('receives authoritative settings query updates without replacing running choices with saved choices', async () => {
  server.use(http.get(url, () => HttpResponse.json(initial())));
  const { queryClient } = renderCitadel(<InstanceSettingsForm />);
  await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  const next = initial();
  next.configuration.passwordMinimumLength = 22;
  next.effectiveConfiguration.passwordMinimumLength = 22;
  next.restartRequired = true;
  await act(async () => {
    queryClient.setQueryData(['getInstallationSettings', {}], { data: next });
  });
  await waitFor(() =>
    expect(screen.getByRole('spinbutton', { name: 'Minimum password length (characters)' })).toHaveValue(22),
  );
  expect(screen.getByText('Saved choice: 22. Currently in use: 15.')).toBeVisible();
});

it('denies direct navigation for a non-administrator before fetching settings', async () => {
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/profile', () => HttpResponse.json({ authorization: { isAdministrator: false } })),
    http.get(url, () => {
      read();
      return HttpResponse.json(initial());
    }),
  );
  renderCitadel(<ResourceView Components={InstanceComponents} type="Instance" />, { route: '/instance' });
  expect(await screen.findByRole('alert')).toHaveTextContent('Administrator access is required.');
  expect(read).not.toHaveBeenCalled();
});
it('preserves deployment log filters and displays the inverse Direct-Agent override accurately', async () => {
  const settings = initial();
  settings.deploymentOverrides.logVerbosity = 'info,sqlx=warn';
  settings.effectiveConfiguration.logVerbosity = 'info,sqlx=warn';
  settings.runningConfiguration.logVerbosity = 'info,sqlx=warn';
  settings.deploymentOverrides.requireDirectAgentHttps = true;
  settings.effectiveConfiguration.requireDirectAgentHttps = true;
  settings.runningConfiguration.requireDirectAgentHttps = true;
  server.use(http.get(url, () => HttpResponse.json(settings)));
  renderCitadel(<InstanceSettingsForm />);
  expect(await screen.findByRole('combobox', { name: 'Log verbosity' })).toBeDisabled();
  expect(screen.getByRole('combobox', { name: 'Log verbosity' })).toHaveTextContent('info,sqlx=warn');
  expect(screen.getByRole('switch', { name: 'Require HTTPS for Direct Agents' })).toBeChecked();
  expect(screen.getByText('CITADEL_CORE_AGENT_ALLOW_INSECURE').parentElement).toHaveTextContent('false');
});
