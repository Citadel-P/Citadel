import { AutomationActionView, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { fireEvent, screen } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { AutomationActionForm } from './form';

const mocks = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn(), mutate: vi.fn() }));

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({
    hasCapability: () => false,
  }),
}));

vi.mock('@/lib/hooks', () => ({
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
  configureAutomationActionEditor: vi.fn(),
  MonacoDiff: () => null,
  MonacoEditor: ({ value, filename, onValueChange }: any) => (
    <textarea aria-label={filename} value={value} onChange={(event) => onValueChange(event.target.value)} />
  ),
}));

vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: () => null,
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

const action: AutomationActionView = {
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

describe('AutomationActionForm licensing', () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.open.mockClear();
    mocks.save.mockClear();
    mocks.mutate.mockClear();
  });

  it('tests edited code and arguments without saving the action', async () => {
    const { user } = renderCitadel(
      <Routes>
        <Route
          path="/automation/edit/:id"
          element={
            <AutomationActionForm
              mode="edit"
              resource={{
                ...action,
                capabilities: { canRead: true, canWrite: true, ...action.capabilities, canExecute: true },
              }}
            />
          }
        />
      </Routes>,
      { route: `/automation/edit/${action.id}` },
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'action.mts' }), {
      target: { value: 'console.log("draft");' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'args.json' }), {
      target: { value: '{"draft":true}' },
    });
    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeEnabled();
    await user.click(screen.getByRole('button', { name: 'Test Draft' }));
    expect(mocks.open).toHaveBeenCalledWith({
      kind: 'automationActionRun',
      payload: {
        id: action.id,
        name: action.name,
        mode: 'test',
        code: 'console.log("draft");',
        argsJson: '{"draft":true}',
      },
    });
    expect(screen.getByRole('textbox', { name: 'action.mts' })).toHaveValue('console.log("draft");');
    expect(mocks.save).not.toHaveBeenCalled();
    expect(mocks.mutate).not.toHaveBeenCalled();
  });

  it.each([
    ['empty code', { code: ' ' }],
    ['invalid arguments', { defaultArgsJson: '{invalid' }],
    ['a running action', { controlState: ResourceControlState.Processing }],
  ])('disables testing with %s', (_reason, changes) => {
    renderCitadel(
      <AutomationActionForm
        mode="edit"
        resource={{
          ...action,
          ...changes,
          capabilities: { canRead: true, canWrite: true, ...action.capabilities, canExecute: true },
        }}
      />,
    );
    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeDisabled();
  });

  it('keeps the cron trigger disabled without Automated Operations', () => {
    renderCitadel(<AutomationActionForm mode="edit" resource={action} />, {
      route: `/automation/${action.id}`,
    });

    const scheduleSwitch = document.querySelector('#automation-action-schedule-enabled');

    expect(scheduleSwitch).toBeInstanceOf(HTMLButtonElement);
    expect(scheduleSwitch).toBeDisabled();
    expect(scheduleSwitch).toHaveAttribute('aria-checked', 'false');
  });

  it('disables draft testing without execute permission', () => {
    renderCitadel(<AutomationActionForm mode="edit" resource={action} />, {
      route: `/automation/${action.id}`,
    });

    expect(screen.getByRole('button', { name: 'Test Draft' })).toBeDisabled();
  });
});
