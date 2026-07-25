import {
  ResourceCapabilities,
  ResourceControlState,
  StackReleaseStatus,
  StackSource,
  StackUpdateState,
  StackView,
} from '@/api/generated/api.types';

export const createResourceCapabilities = (
  overrides: Partial<ResourceCapabilities> = {},
): ResourceCapabilities => ({
  canRead: true,
  canWrite: true,
  canExecute: true,
  ...overrides,
});

export const createStack = (overrides: Partial<StackView> = {}): StackView => ({
  id: '00000000-0000-0000-0000-000000000001',
  name: 'api',
  description: null,
  stackSource: StackSource.WebEditor,
  stackUpdateState: {
    $type: 'WebEditor',
    recreateStackOnNewImageState: { autoUpdateStates: [] },
  } as StackUpdateState,
  driftPolicy: {
    mode: 'Disabled',
    alertOnDrift: false,
    markDegraded: false,
    autoStartStoppedContainers: false,
    autoResumePausedContainers: false,
    removeExtraContainers: false,
  },
  status: StackReleaseStatus.Healthy,
  createdAt: '2026-01-01T00:00:00.000Z',
  createdByActorId: '00000000-0000-0000-0000-000000000010',
  controlState: ResourceControlState.Idle,
  currentStackReleaseId: '00000000-0000-0000-0000-000000000100',
  platformId: '00000000-0000-0000-0000-000000000200',
  tags: [],
  ...overrides,
});
