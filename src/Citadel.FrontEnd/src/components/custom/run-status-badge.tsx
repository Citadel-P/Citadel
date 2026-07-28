import { BackupRestoreStatus, BackupRunStatus, BuildRunStatus } from '@/api/generated/api.types';
import { Badge } from '@/components/ui/badge';

type RunStatus = BackupRunStatus | BackupRestoreStatus | BuildRunStatus;

const STATUS_CONFIG: Record<string, { className: string; label: string }> = {
  [BackupRunStatus.Queued]: { className: 'bg-orange-200/25 text-orange-500', label: 'Queued' },
  [BackupRunStatus.Preparing]: { className: 'bg-blue-200/25 text-blue-700', label: 'Preparing' },
  [BackupRunStatus.Running]: { className: 'bg-blue-200/25 text-blue-700', label: 'Running' },
  [BackupRunStatus.ApplyingRetention]: {
    className: 'bg-blue-200/25 text-blue-700',
    label: 'Applying retention',
  },
  [BackupRunStatus.Succeeded]: { className: 'bg-green-200/25 text-green-700', label: 'Succeeded' },
  [BackupRunStatus.SucceededWithWarnings]: {
    className: 'bg-orange-200/25 text-orange-500',
    label: 'Succeeded with warnings',
  },
  [BackupRunStatus.Failed]: { className: 'bg-red-200/25 text-red-700', label: 'Failed' },
  [BackupRunStatus.TimedOut]: { className: 'bg-red-200/25 text-red-700', label: 'Timed out' },
  [BackupRunStatus.Interrupted]: { className: 'bg-red-200/25 text-red-700', label: 'Interrupted' },
  [BackupRunStatus.Cancelled]: { className: 'bg-muted text-muted-foreground', label: 'Cancelled' },
  [BackupRunStatus.Rejected]: { className: 'bg-red-200/25 text-red-700', label: 'Rejected' },
};

export function RunStatusBadge({ status }: { status: RunStatus }) {
  const config = STATUS_CONFIG[status] ?? { className: 'bg-muted text-muted-foreground', label: status };

  return <Badge className={config.className}>{config.label}</Badge>;
}
