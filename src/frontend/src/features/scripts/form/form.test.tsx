import { AuthorizedScript, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { fireEvent, screen } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { ScriptForm } from './form';

const mocks = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn(), mutate: vi.fn(), read: vi.fn(), paid: false }));

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({
    hasCapability: () => mocks.paid,
  }),
}));

vi.mock('@/lib/hooks', () => ({
  useRead: (...args: unknown[]) => mocks.read(...args) ?? { data: undefined, isFetching: false },
  useMutate: () => ({ mutateAsync: mocks.mutate }),
  useSaveResource: () => ({
    save: mocks.save,
    isPending: false,
  }),
}));

vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({ open: mocks.open }),
}));

vi.mock('@/lib/monaco', () => ({
  configureScriptEditor: vi.fn(),
  MonacoDiff: () => null,
  MonacoEditor: ({ value, filename, onValueChange }: any) => (
    <textarea aria-label={filename} value={value} onChange={(event) => onValueChange(event.target.value)} />
  ),
}));

vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: ({ placeholder }: { placeholder?: string }) => (placeholder ? <div>{placeholder}</div> : null),
}));

vi.mock('@/components/custom/timezone-select', () => ({
  TimezoneSelectField: () => null,
}));

vi.mock('@/components/custom/webhook-config-field', () => ({
  WebhookConfigField: () => null,
}));

vi.mock('@/features/tags/components', () => ({
  ResourceTagSelector: () => null,
}));

const script: AuthorizedScript = {
  executor: { kind: 'Core' },
  id: '019fb442-8b08-7fea-b21d-6ea46f42da21',
  name: 'Prune images',
  description: null,
  code: 'console.log("prune");',
  defaultArgsJson: '{}',
  enabled: false,
  scheduleEnabled: false,
  scheduleCron: '0 12 * * *',
  scheduleTimeZone: 'UTC',
  webhook: null,
  timeoutSeconds: 300,
  alertOnFailure: true,
  runAsActorId: '019fb442-8b08-7fea-b21d-6ea46f42da22',
  lastScheduledRunAt: null,
  controlState: ResourceControlState.Idle,
  currentRunId: null,
  rowVersion: 1,
  latestRun: null,
  createdByActorId: '019fb442-8b08-7fea-b21d-6ea46f42da23',
  createdAt: '2026-07-30T00:00:00Z',
  updatedAt: '2026-07-30T00:00:00Z',
  tags: [],
  capabilities: {
    canRead: true,
    canWrite: true,
    canExecute: false,
  },
};

describe('ScriptForm licensing', () => {
  beforeEach(() => {
    mocks.paid = false;
    localStorage.clear();
    mocks.open.mockClear();
    mocks.save.mockClear();
    mocks.mutate.mockClear();
    mocks.read.mockReset();
  });

  it('locks only Automatic without the entitlement', async () => {
    const { user } = renderCitadel(<ScriptForm mode="edit" resource={script} />);
    await user.click(screen.getByRole('combobox', { name: 'Run on' }));
    expect(screen.getByRole('option', { name: /Automatic/ })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('option', { name: /Core/ })).not.toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('option', { name: /Specific agent/ })).not.toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('option', { name: /Automatic/ })).toHaveTextContent('Team');
  });

  it('keeps saved Automatic visible after downgrade and allows switching to Core', async () => {
    const { user } = renderCitadel(
      <ScriptForm mode="edit" resource={{ ...script, executor: { kind: 'Automatic' } }} />,
    );
    expect(
      screen.getByText('Automatic requires Team. Choose Core or a specific agent to run this script.'),
    ).toBeVisible();
    await user.click(screen.getByRole('combobox', { name: 'Run on' }));
    await user.click(screen.getByRole('option', { name: /Core/ }));
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({ executor: { kind: 'Core' } }));
  });

  it('describes placement choices and saves Automatic without choosing agents', async () => {
    mocks.paid = true;
    const { user } = renderCitadel(<ScriptForm mode="edit" resource={script} />);
    await user.click(screen.getByRole('combobox', { name: 'Run on' }));
    expect(screen.getByText('Run on the Citadel Core server.')).toBeVisible();
    expect(screen.getByText('Always run on the agent you select.')).toBeVisible();
    expect(screen.getByText('Let Citadel choose an eligible agent with available capacity.')).toBeVisible();
    await user.click(screen.getByRole('option', { name: /Specific agent/ }));
    expect(screen.getByText('Select agent')).toBeVisible();
    await user.click(screen.getByRole('combobox', { name: 'Run on' }));
    await user.click(screen.getByRole('option', { name: /Automatic/ }));
    expect(screen.queryByText('Select agent')).not.toBeInTheDocument();
    expect(screen.queryByText('Select eligible agents')).not.toBeInTheDocument();
    await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({ executor: { kind: 'Automatic' } }));
  });

  it('tests edited code and arguments without saving the script', async () => {
    const { user } = renderCitadel(
      <Routes>
        <Route
          path="/scripts/edit/:id"
          element={
            <ScriptForm
              mode="edit"
              resource={{
                ...script,
                capabilities: { ...script.capabilities, canRead: true, canWrite: true, canExecute: true },
              }}
            />
          }
        />
      </Routes>,
      { route: `/scripts/edit/${script.id}` },
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'script.mts' }), {
      target: { value: 'console.log("draft");' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'args.json' }), {
      target: { value: '{"draft":true}' },
    });
    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeEnabled();
    await user.click(screen.getByRole('button', { name: 'Test Draft' }));
    expect(mocks.open).toHaveBeenCalledWith({
      kind: 'scriptRun',
      payload: {
        id: script.id,
        name: script.name,
        mode: 'test',
        code: 'console.log("draft");',
        argsJson: '{"draft":true}',
      },
    });
    expect(screen.getByRole('textbox', { name: 'script.mts' })).toHaveValue('console.log("draft");');
    expect(mocks.save).not.toHaveBeenCalled();
    expect(mocks.mutate).not.toHaveBeenCalled();
  });

  it.each([
    ['empty code', { code: ' ' }],
    ['invalid arguments', { defaultArgsJson: '{invalid' }],
    ['a running script', { controlState: ResourceControlState.Processing }],
  ])('disables testing with %s', (_reason, changes) => {
    renderCitadel(
      <ScriptForm
        mode="edit"
        resource={{
          ...script,
          ...changes,
          capabilities: { ...script.capabilities, canRead: true, canWrite: true, canExecute: true },
        }}
      />,
    );
    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeDisabled();
  });

  it('keeps the cron trigger disabled without Automated Operations', () => {
    renderCitadel(<ScriptForm mode="edit" resource={script} />, {
      route: `/scripts/${script.id}`,
    });

    const scheduleSwitch = document.querySelector('#script-schedule-enabled');

    expect(scheduleSwitch).toBeInstanceOf(HTMLButtonElement);
    expect(scheduleSwitch).toBeDisabled();
    expect(scheduleSwitch).toHaveAttribute('aria-checked', 'false');
  });

  it('disables draft testing without execute permission', () => {
    renderCitadel(<ScriptForm mode="edit" resource={script} />, {
      route: `/scripts/${script.id}`,
    });

    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeDisabled();
  });
});

it('keeps copied scripts inactive and reviews the edited name before saving', async () => {
  mocks.save.mockClear();
  mocks.read.mockReturnValue({
    data: {
      data: {
        sourceName: script.name,
        draft: {
          name: 'Prune images Copy',
          description: null,
          code: script.code,
          defaultArgsJson: '{}',
          duplicateSourceId: script.id,
          enabled: false,
          scheduleEnabled: false,
          scheduleCron: '0 12 * * *',
          scheduleTimeZone: 'UTC',
          webhook: null,
          timeoutSeconds: 300,
          alertOnFailure: true,
          runAsActorId: null,
          tagIds: [],
        },
      },
    },
    isFetching: false,
  });
  const { user } = renderCitadel(<ScriptForm mode="add" />, {
    route: `/scripts/add?duplicateFrom=${script.id}`,
  });
  const name = await screen.findByDisplayValue('Prune images Copy');
  await user.clear(name);
  await user.type(name, 'Prune staging');
  for (const control of screen.getAllByRole('switch', { name: 'Enabled', checked: false }))
    expect(control).toBeDisabled();
  await user.click(screen.getAllByRole('button', { name: 'Review copy' })[0]);
  expect(await screen.findByRole('dialog')).toHaveTextContent('Prune staging');
  expect(mocks.save).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Create configuration copy' }));
  expect(mocks.save).toHaveBeenCalledWith(
    expect.objectContaining({
      name: 'Prune staging',
      duplicateSourceId: script.id,
      enabled: false,
      scheduleEnabled: false,
      runAsActorId: '',
      webhook: null,
    }),
  );
  expect(Object.keys(localStorage).some((key) => key.includes(script.id))).toBe(false);
});
