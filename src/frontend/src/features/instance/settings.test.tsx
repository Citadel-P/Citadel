import { screen, waitFor, act, fireEvent } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { LogFormatView, MfaPolicy, type InstanceConfigurationView } from '@/api/generated/api.types';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { InstanceSettingsForm } from './settings';
import { InstanceComponents } from './index';
import { ResourceView } from '@/pages/resource-view';
import { instanceSections } from './fields';
import { instanceSettings as initial } from '@/test/factories/instance';

const url = 'http://localhost/api/v1/installation/settings';

function InstanceSettingsWithStatus() {
  return (
    <>
      {InstanceComponents.SubHeader && <InstanceComponents.SubHeader />}
      <InstanceSettingsForm />
    </>
  );
}

beforeEach(() => {
  localStorage.clear();
});

it('keeps MFA as a draft, resets it, and saves it with other changed fields in one request', async () => {
  let settings = initial();
  const save = vi.fn();
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.patch(url, async ({ request }) => {
      const body = (await request.json()) as {
        mfaPolicy: MfaPolicy;
        configuration: Partial<InstanceConfigurationView>;
      };
      save(body);
      settings = {
        ...settings,
        mfaPolicy: body.mfaPolicy,
        configuration: { ...settings.configuration, ...body.configuration },
        effectiveConfiguration: { ...settings.effectiveConfiguration, ...body.configuration },
        restartRequired: true,
      };
      return HttpResponse.json(settings);
    }),
  );
  const { user } = renderCitadel(<InstanceSettingsWithStatus />);
  const policy = await screen.findByRole('combobox', { name: 'Two-factor authentication policy' });
  await user.click(policy);
  await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
  expect(policy).toHaveTextContent('Required for administrators');
  expect(save).not.toHaveBeenCalled();
  await user.click(screen.getAllByRole('button', { name: 'Reset' })[0]);
  expect(policy).toHaveTextContent('Optional');
  expect(save).not.toHaveBeenCalled();
  await user.click(policy);
  await user.click(screen.getByRole('option', { name: 'Required for administrators' }));
  const minimum = screen.getByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '20');
  expect(save).not.toHaveBeenCalled();
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() =>
    expect(save).toHaveBeenCalledExactlyOnceWith({
      mfaPolicy: MfaPolicy.RequiredForAdministrators,
      configuration: { passwordMinimumLength: 20 },
    }),
  );
  await waitFor(() => expect(policy).toBeEnabled());
  expect(policy).toHaveTextContent('Required for administrators');
});

it('retains the MFA draft after a failed save and resets to the saved policy', async () => {
  server.use(
    http.get(url, () => HttpResponse.json(initial())),
    http.patch(url, () => new HttpResponse(null, { status: 500 })),
  );
  const { user } = renderCitadel(<InstanceSettingsWithStatus />);
  const policy = await screen.findByRole('combobox', { name: 'Two-factor authentication policy' });
  await user.click(policy);
  await user.click(screen.getByRole('option', { name: 'Required for everyone' }));
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  expect(await screen.findByText(/Could not save instance settings/)).toBeVisible();
  expect(policy).toHaveTextContent('Required for everyone');
  await user.click(screen.getAllByRole('button', { name: 'Reset' })[0]);
  expect(policy).toHaveTextContent('Optional');
});

it('disables the MFA selector when the deployment requires MFA for everyone', async () => {
  server.use(
    http.get(url, () =>
      HttpResponse.json({
        ...initial(),
        mfaPolicy: MfaPolicy.RequiredForAllUsers,
        minimumMfaPolicy: MfaPolicy.RequiredForAllUsers,
      }),
    ),
  );
  renderCitadel(<InstanceSettingsForm />);
  expect(await screen.findByRole('combobox', { name: 'Two-factor authentication policy' })).toBeDisabled();
});

it('registers an administrator page and exposes all 26 controls with server defaults and units', async () => {
  server.use(http.get(url, () => HttpResponse.json(initial())));
  renderCitadel(<InstanceSettingsWithStatus />);
  expect(InstanceComponents.access).toBe('administrator');
  expect(instanceSections.map((s) => s.label)).toEqual([
    'Security',
    'Monitoring',
    'Scripts',
    'Builds',
    'Backups',
    'Runtime',
  ]);
  expect(instanceSections.flatMap((s) => s.fields)).toHaveLength(25);
  expect(await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' })).toHaveValue(15);
  expect(screen.getByRole('spinbutton', { name: 'Browser session lifetime (days)' })).toHaveValue(30);
  expect(screen.getByRole('combobox', { name: 'Two-factor authentication policy' })).toHaveTextContent('Optional');
  expect(screen.getByText('MFA changes apply on save. Restart Core to apply other saved settings.')).toBeVisible();
  expect(screen.queryByText('Applies after a Core restart.')).not.toBeInTheDocument();
  expect(screen.queryByText('Saves on selection')).not.toBeInTheDocument();
  for (const field of instanceSections.flatMap((section) => section.fields)) {
    expect(
      screen.getByRole('link', { name: `Environment variable ${field.environment}: open documentation` }),
    ).toHaveAttribute(
      'href',
      `https://docs.citadelplane.com/reference/environment-variables/#${field.environment.toLowerCase().replaceAll('_', '-')}`,
    );
  }
  expect(screen.queryByText('CITADEL_PASSWORDS_MINIMUM_LENGTH')).not.toBeInTheDocument();
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
  const { user } = renderCitadel(<InstanceSettingsWithStatus />);
  const minimum = await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '20');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(requests).toEqual([{ configuration: { passwordMinimumLength: 20 } }]));
  expect((await screen.findByText('Restart required')).parentElement).toHaveTextContent(
    'Restart required: saved settings differ from the values currently in use.',
  );
  expect(
    screen.queryByText('MFA changes apply on save. Restart Core to apply other saved settings.'),
  ).not.toBeInTheDocument();
  expect(screen.getByText('Saved choice: 20. Currently in use: 15.')).toBeVisible();
});

it('shows save errors in the page status and clears them after a successful retry', async () => {
  let fail = true;
  server.use(
    http.get(url, () => HttpResponse.json(initial())),
    http.patch(url, () =>
      fail
        ? HttpResponse.json({ title: 'Save failed', detail: 'Please retry.' }, { status: 500 })
        : HttpResponse.json(initial()),
    ),
  );
  const { user } = renderCitadel(<InstanceSettingsWithStatus />);
  const minimum = await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '20');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  expect(await screen.findByText(/Could not save instance settings/)).toBeVisible();
  fail = false;
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(screen.queryByText(/Could not save instance settings/)).not.toBeInTheDocument());
});

it('displays explicit deployment overrides and prevents conflicting edits', async () => {
  const settings = initial();
  settings.deploymentOverrides.passwordMinimumLength = 18;
  settings.effectiveConfiguration.passwordMinimumLength = 18;
  settings.runningConfiguration.passwordMinimumLength = 18;
  server.use(http.get(url, () => HttpResponse.json(settings)));
  renderCitadel(<InstanceSettingsForm />);
  const minimum = await screen.findByLabelText('Minimum password length (characters)');
  expect(minimum).toHaveTextContent('18');
  expect(screen.queryByRole('spinbutton', { name: 'Minimum password length (characters)' })).not.toBeInTheDocument();
  expect(screen.getByText('Managed by environment')).toBeVisible();
  expect(screen.getByText(/Recreate the Core container/)).toBeVisible();
  expect(screen.getByRole('link', { name: /Environment variable CITADEL_PASSWORDS_MINIMUM_LENGTH/ })).toBeVisible();
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
  settings.deploymentOverrides.logFormat = LogFormatView.Text;
  settings.deploymentOverrides.requireDirectAgentHttps = true;
  settings.effectiveConfiguration.requireDirectAgentHttps = true;
  settings.runningConfiguration.requireDirectAgentHttps = true;
  server.use(http.get(url, () => HttpResponse.json(settings)));
  renderCitadel(<InstanceSettingsForm />);
  expect(await screen.findByLabelText('Log verbosity')).toHaveTextContent('info,sqlx=warn');
  expect(screen.getByLabelText('Log output format')).toHaveTextContent('Text');
  expect(screen.queryByRole('combobox', { name: 'Log verbosity' })).not.toBeInTheDocument();
  expect(screen.queryByRole('combobox', { name: 'Log output format' })).not.toBeInTheDocument();
  expect(screen.getByLabelText('Require HTTPS for Direct Agents')).toHaveTextContent('Enabled');
  expect(screen.queryByRole('switch', { name: 'Require HTTPS for Direct Agents' })).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: /Environment variable CITADEL_CORE_AGENT_ALLOW_INSECURE/ })).toBeVisible();
  expect(screen.getByText(/Environment value: false/)).toBeVisible();
});

it('reveals the environment variable on hover and keyboard focus', async () => {
  server.use(http.get(url, () => HttpResponse.json(initial())));
  const { user } = renderCitadel(<InstanceSettingsForm />);
  const reference = await screen.findByRole('link', { name: /Environment variable CITADEL_PASSWORDS_MINIMUM_LENGTH/ });
  await user.hover(reference);
  expect(await screen.findByRole('tooltip')).toHaveTextContent('CITADEL_PASSWORDS_MINIMUM_LENGTH');
  await user.unhover(reference);
  await user.keyboard('{Escape}');
  await act(async () => reference.focus());
  expect(await screen.findByRole('tooltip')).toHaveTextContent('CITADEL_PASSWORDS_MINIMUM_LENGTH');
});

it('shows Back to top for the Instance scroll container and scrolls it to the start', async () => {
  server.use(http.get(url, () => HttpResponse.json(initial())));
  renderCitadel(
    <div id="main-scroll-container">
      {InstanceComponents.SubHeader && <InstanceComponents.SubHeader />}
      <InstanceSettingsForm />
    </div>,
  );
  await screen.findByRole('spinbutton', { name: 'Minimum password length (characters)' });
  expect(screen.queryByRole('button', { name: 'Back to top' })).not.toBeInTheDocument();
  const container = document.getElementById('main-scroll-container')!;
  container.scrollTo = vi.fn();
  fireEvent.scroll(container, { target: { scrollTop: 500 } });
  fireEvent.click(await screen.findByRole('button', { name: 'Back to top' }));
  expect(container.scrollTo).toHaveBeenCalledWith(expect.objectContaining({ top: 0 }));
  fireEvent.scroll(container, { target: { scrollTop: 0 } });
  expect(screen.queryByRole('button', { name: 'Back to top' })).not.toBeInTheDocument();
});

it('keeps unmanaged fields editable and excludes environment values from the saved patch', async () => {
  const settings = initial();
  settings.deploymentOverrides.openapiEnabled = false;
  settings.deploymentOverrides.logVerbosity = 'info';
  const requests: unknown[] = [];
  server.use(
    http.get(url, () => HttpResponse.json(settings)),
    http.patch(url, async ({ request }) => {
      requests.push(await request.json());
      return HttpResponse.json(settings);
    }),
  );
  const { user } = renderCitadel(<InstanceSettingsForm />);
  expect(await screen.findByLabelText('Serve Swagger UI and OpenAPI documents')).toHaveTextContent('Disabled');
  expect(screen.getByLabelText('Log verbosity')).toHaveTextContent('info');
  expect(screen.getByRole('combobox', { name: 'Log output format' })).toBeEnabled();
  const minimum = screen.getByRole('spinbutton', { name: 'Minimum password length (characters)' });
  await user.clear(minimum);
  await user.type(minimum, '20');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(requests).toEqual([{ configuration: { passwordMinimumLength: 20 } }]));
});
