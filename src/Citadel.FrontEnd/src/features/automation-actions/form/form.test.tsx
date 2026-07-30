import { AutomationActionView, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { AutomationActionForm } from './form';

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({
    hasCapability: () => false,
  }),
}));

vi.mock('@/lib/hooks', () => ({
  useMutate: () => ({ mutateAsync: vi.fn() }),
  useSaveResource: () => ({
    save: vi.fn(),
    isPending: false,
  }),
}));

vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({ open: vi.fn() }),
}));

vi.mock('@/lib/monaco', () => ({
  configureAutomationActionEditor: vi.fn(),
  MonacoDiff: () => null,
  MonacoEditor: () => <div />,
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
};

describe('AutomationActionForm licensing', () => {
  it('keeps the cron trigger disabled without Automated Operations', () => {
    renderCitadel(<AutomationActionForm mode="edit" resource={action} />, {
      route: `/automation/${action.id}`,
    });

    const scheduleSwitch = document.querySelector('#automation-action-schedule-enabled');

    expect(scheduleSwitch).toBeInstanceOf(HTMLButtonElement);
    expect(scheduleSwitch).toBeDisabled();
    expect(scheduleSwitch).toHaveAttribute('aria-checked', 'false');
  });
});
