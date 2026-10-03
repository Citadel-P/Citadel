import { BackupRunView } from '@/api/generated/api.types';
import { describe, expect, it } from 'vitest';
import { getBackupPolicyLastRunAt } from './table';

describe('getBackupPolicyLastRunAt', () => {
  it('uses the completion time for a finished manual run', () => {
    expect(
      getBackupPolicyLastRunAt({
        latestRun: {
          queuedAt: '2026-07-28T08:00:00Z',
          completedAt: '2026-07-28T08:02:00Z',
        } as BackupRunView,
      }),
    ).toBe('2026-07-28T08:02:00Z');
  });

  it('uses the queue time while the latest run is active', () => {
    expect(
      getBackupPolicyLastRunAt({
        latestRun: {
          queuedAt: '2026-07-28T08:00:00Z',
          completedAt: null,
        } as BackupRunView,
      }),
    ).toBe('2026-07-28T08:00:00Z');
  });
});
