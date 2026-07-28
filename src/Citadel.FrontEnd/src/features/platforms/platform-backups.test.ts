import { PlatformBackupSummaryView } from '@/api/generated/api.types';
import { getBackupMetric, getBackupSourceStates } from './platform-backups';

describe('platform backup summary', () => {
  it('keeps backups visible when the platform has no policies', () => {
    const summary = createSummary();

    expect(getBackupMetric(summary, false)).toEqual({
      value: 0,
      detail: 'No policies',
      detailTitle: '0 volume, 0 stack, 0 deployment',
    });
  });

  it('reports policy health and source types without treating backups as volume-only', () => {
    const summary = createSummary({
      policyCount: 4,
      enabledPolicyCount: 3,
      dockerVolumePolicyCount: 1,
      stackPolicyCount: 2,
      deploymentPolicyCount: 1,
      attentionPolicyCount: 1,
    });

    expect(getBackupMetric(summary, false)).toMatchObject({
      value: 4,
      detail: '1 needs attention',
      detailTitle: '1 volume, 2 stack, 1 deployment',
    });
    expect(getBackupSourceStates(summary).map((state) => [state.label, state.value])).toEqual([
      ['Volume policies', 1],
      ['Stack policies', 2],
      ['Deployment policies', 1],
    ]);
  });
});

const createSummary = (overrides: Partial<PlatformBackupSummaryView> = {}): PlatformBackupSummaryView => ({
  platformId: 'platform-1',
  policyCount: 0,
  enabledPolicyCount: 0,
  dockerVolumePolicyCount: 0,
  stackPolicyCount: 0,
  deploymentPolicyCount: 0,
  attentionPolicyCount: 0,
  lastRunStatus: null,
  lastRunAt: null,
  ...overrides,
});
