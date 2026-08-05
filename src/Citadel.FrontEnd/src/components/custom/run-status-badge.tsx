import { BackupRestoreStatus, BackupRunStatus, BuildRunStatus } from '@/api/generated/api.types';
import { StateBadge } from './state-badge';

type RunStatus = BackupRunStatus | BackupRestoreStatus | BuildRunStatus;

const STATUS_LABELS: Record<string, string> = {
  [BackupRunStatus.ApplyingRetention]: 'Applying retention',
  [BackupRunStatus.SucceededWithWarnings]: 'Succeeded with warnings',
  [BackupRunStatus.TimedOut]: 'Timed out',
};

export function RunStatusBadge({ status }: { status: RunStatus }) {
  return <StateBadge value={status} kind="run" label={STATUS_LABELS[status] ?? status} />;
}
