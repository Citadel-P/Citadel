import { screen } from '@testing-library/react';
import {
  LicenseCapability,
  PlatformType,
  ResourceControlState,
  StackDriftMode,
  StackDriftReport,
  StackReleaseStatus,
  StackView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { hasActionableStackDrift, syncAction } from './actions';

const mocks = vi.hoisted(() => ({
  licensed: true,
  report: undefined as StackDriftReport | undefined,
  refetch: vi.fn(),
  mutateAsync: vi.fn(),
}));

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({
    hasCapability: (capability: LicenseCapability) =>
      capability === LicenseCapability.OperationalGuardrails && mocks.licensed,
  }),
}));
vi.mock('@/lib/hooks', () => ({
  useRead: () => ({ data: { data: mocks.report }, refetch: mocks.refetch, isFetching: false }),
  useMutate: () => ({ mutateAsync: mocks.mutateAsync, isPending: false }),
}));

const stack = {
  id: 'stack',
  platformType: PlatformType.Docker,
  status: StackReleaseStatus.Degraded,
  controlState: ResourceControlState.Idle,
  driftPolicy: {
    mode: StackDriftMode.DetectOnly,
    alertOnDrift: true,
    markDegraded: true,
    autoStartStoppedContainers: false,
    autoResumePausedContainers: false,
    removeExtraContainers: false,
  },
} as StackView;
const report: StackDriftReport = {
  stackId: 'stack',
  platformId: 'platform',
  hasDrift: true,
  hasAutoFixableDrift: false,
  hasStructuralDrift: false,
  drifts: [{ $type: 'ContainerStopped', containerId: 'container', serviceName: 'web' }],
};

function Reconcile() {
  if (syncAction.type !== 'command' || !syncAction.useHandler) throw new Error('Expected reconcile handler');
  const handler = syncAction.useHandler({ resources: stack });
  return (
    <button disabled={!handler.canExecute} onClick={handler.run}>
      Reconcile drift
    </button>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.licensed = true;
  mocks.report = report;
});

it('enables manual repair in DetectOnly with a license, but rechecks drift before executing', async () => {
  mocks.refetch.mockResolvedValue({ data: { data: { ...report, hasDrift: false, drifts: [] } } });
  const { user } = renderCitadel(<Reconcile />);
  const button = screen.getByRole('button', { name: 'Reconcile drift' });
  expect(button).toBeEnabled();
  await user.click(button);
  expect(mocks.refetch).toHaveBeenCalledOnce();
  expect(mocks.mutateAsync).not.toHaveBeenCalled();
});

it('keeps manual repair disabled without OperationalGuardrails', () => {
  mocks.licensed = false;
  renderCitadel(<Reconcile />);
  expect(screen.getByRole('button', { name: 'Reconcile drift' })).toBeDisabled();
});

it.each([undefined, { ...report, hasDrift: false, drifts: [] }, { ...report, hasStructuralDrift: true }])(
  'disables reconciliation for absent, resolved, or structural drift',
  (value) => {
    mocks.report = value;
    renderCitadel(<Reconcile />);
    expect(screen.getByRole('button', { name: 'Reconcile drift' })).toBeDisabled();
  },
);

it('allows manual resume but preserves explicit opt-in for destructive cleanup', () => {
  expect(
    hasActionableStackDrift(stack, {
      ...report,
      drifts: [{ $type: 'ContainerPaused', containerId: 'container', serviceName: 'web' }],
    }),
  ).toBe(true);
  const extra: StackDriftReport = {
    ...report,
    drifts: [{ $type: 'ExtraContainer', containerId: 'extra', serviceName: 'old' }],
  };
  expect(hasActionableStackDrift(stack, extra)).toBe(false);
  expect(
    hasActionableStackDrift({ ...stack, driftPolicy: { ...stack.driftPolicy, removeExtraContainers: true } }, extra),
  ).toBe(true);
  expect(
    hasActionableStackDrift({ ...stack, driftPolicy: { ...stack.driftPolicy, mode: StackDriftMode.Disabled } }, report),
  ).toBe(false);
});
