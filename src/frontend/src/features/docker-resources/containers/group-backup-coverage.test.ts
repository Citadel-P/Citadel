import {
  BackupCoverageStatus as Status,
  type ContainerView,
  type ResourceBackupSummaryView,
} from '@/api/generated/api.types';
import { groupBackupCoverage } from './group-backup-coverage';

const coverage = (name: string, status = Status.Protected): ResourceBackupSummaryView => ({
  platformId: 'platform',
  status,
  unavailable: false,
  hasResourcePolicy: false,
  canConfigure: true,
  canConfigureBindMount: true,
  policies: [],
  warnings: [],
  reasons: [],
  volumes: [{ name, status, includedInResourcePolicy: false, policies: [] }],
});
const container = (backupCoverage?: ResourceBackupSummaryView): ContainerView =>
  ({ platformId: 'platform', backupCoverage }) as ContainerView;

it('counts shared storage once while preserving the coverage of every distinct target', () => {
  const entries = [
    container(coverage('shared')),
    container(coverage('shared')),
    container(coverage('uploads', Status.Unprotected)),
  ];
  const original = structuredClone(entries);
  const result = groupBackupCoverage(entries)!;
  expect(result.volumes.map((volume) => volume.name)).toEqual(['shared', 'uploads']);
  expect(result.status).toBe(Status.Warning);
  expect(result.hasResourcePolicy).toBe(false);
  expect(entries).toEqual(original);
});

it('keeps node identity and bind-mount identity distinct', () => {
  const first = coverage('data');
  const second = coverage('data');
  const bind = coverage('data');
  first.volumes[0].dockerNodeId = 'node-a';
  second.volumes[0].dockerNodeId = 'node-b';
  bind.volumes[0].dockerNodeId = 'node-a';
  bind.volumes[0].hostPath = '/srv/data';
  expect(groupBackupCoverage([container(first), container(second), container(bind)])?.volumes).toHaveLength(3);
});

it.each([Status.Protected, Status.Unprotected, Status.Warning, Status.Failed, Status.NotApplicable])(
  'preserves server status %s',
  (status) => {
    expect(groupBackupCoverage([container(coverage('one', status))])?.status).toBe(status);
  },
);

it('does not treat missing, stale, unavailable, or cross-platform evidence as protected', () => {
  const known = container(coverage('data'));
  for (const other of [
    container(),
    { ...known, projectionStaleSince: 1 },
    container({ ...coverage('data'), unavailable: true }),
    { ...known, platformId: 'other' },
    container({ ...coverage('data'), platformId: 'other' }),
  ]) {
    expect(groupBackupCoverage([known, other])).toBeUndefined();
  }
  expect(groupBackupCoverage([])).toBeUndefined();
});

it('preserves failures and treats contradictory shared-target observations conservatively', () => {
  expect(
    groupBackupCoverage([container(coverage('shared')), container(coverage('shared', Status.Unprotected))])?.volumes[0]
      .status,
  ).toBe(Status.Warning);
  expect(
    groupBackupCoverage([container(coverage('shared')), container(coverage('shared', Status.Failed))])?.status,
  ).toBe(Status.Failed);
  const unknown = coverage('shared');
  unknown.status = null;
  unknown.volumes[0].status = null;
  expect(groupBackupCoverage([container(coverage('shared')), container(unknown)])?.status).toBeNull();
});

it('retains warnings and policy identities without widening configuration permissions', () => {
  const first = coverage('first');
  first.policies = [{ id: 'policy', name: 'Daily', canWrite: true, enabled: true, scheduleConfigured: true }];
  first.warnings = ['Check storage'];
  first.volumes[0].policies = first.policies;
  const second = {
    ...coverage('second'),
    policies: [{ ...first.policies[0], canWrite: false }],
    canConfigure: false,
    canConfigureBindMount: false,
  };
  const result = groupBackupCoverage([container(first), container(second)])!;
  expect(result.policies).toHaveLength(1);
  expect(result.policies[0].canWrite).toBe(false);
  expect(result.canConfigure).toBe(false);
  expect(result.canConfigureBindMount).toBe(false);
  expect(result.warnings).toEqual(['Check storage']);
});
