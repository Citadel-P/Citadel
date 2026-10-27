import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { BackupCoverageStatus } from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { CircleHelp, ShieldCheck, ShieldOff, ShieldAlert } from 'lucide-react';
import { Coverage, backupConfigurationIssue } from './coverage';

export function CoverageIndicator({
  coverage,
  loading = false,
  label = false,
  volumeName,
  kind = 'volume',
}: {
  coverage: Coverage;
  loading?: boolean;
  label?: boolean;
  volumeName: string;
  kind?: 'volume' | 'bind mount';
}) {
  const formatDateTime = useProfileDateTimeFormatter();
  const state = loading ? undefined : coverage.status;
  const detail = loading
    ? { Icon: CircleHelp, label: 'Loading', description: 'Loading backup coverage.', color: 'text-muted-foreground' }
    : state === BackupCoverageStatus.Protected
      ? {
          Icon: ShieldCheck,
          label: 'Protected',
          description: `This ${kind} has an enabled schedule and a successful available backup.`,
          color: 'text-emerald-600 dark:text-emerald-400',
        }
      : state === BackupCoverageStatus.Unprotected
        ? {
            Icon: ShieldOff,
            label: 'Unprotected',
            description: `No backup policy covers this ${kind}. Coverage reflects policies you can access.`,
            color: 'text-amber-700 dark:text-amber-400',
          }
        : state === BackupCoverageStatus.Failed
          ? {
              Icon: ShieldAlert,
              label: 'Failed',
              description: `A backup of this ${kind} failed. Review the policy runs.`,
              color: 'text-destructive',
            }
          : state === BackupCoverageStatus.Warning
            ? {
                Icon: ShieldAlert,
                label: backupConfigurationIssue(coverage.policies) ?? 'Needs attention',
                description: backupConfigurationIssue(coverage.policies)
                  ? `${backupConfigurationIssue(coverage.policies)}. Configure an enabled schedule for ongoing protection.`
                  : 'A backup policy is configured, but protection needs attention. Check whether it is enabled, its repository is ready, and a backup has succeeded.',
                color: 'text-amber-700 dark:text-amber-400',
              }
            : state === BackupCoverageStatus.NotApplicable
              ? {
                  Icon: CircleHelp,
                  label: 'Not covered',
                  description:
                    kind === 'bind mount'
                      ? 'This host path is not supported for backup.'
                      : 'This storage is not covered by volume backups.',
                  color: 'text-muted-foreground',
                }
              : {
                  Icon: CircleHelp,
                  label: 'Unavailable',
                  description: 'Backup coverage is unavailable.',
                  color: 'text-muted-foreground',
                };
  const Icon = detail.Icon;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={`${volumeName}: ${detail.label}`}
          className={cn(
            'inline-flex shrink-0 items-center gap-1.5 rounded-sm text-xs font-normal outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring',
            detail.color,
          )}>
          <Icon className="size-3.5" aria-hidden="true" />
          {label && <span>{detail.label}</span>}
        </button>
      </TooltipTrigger>
      <TooltipContent className="max-w-xs space-y-1">
        <p>{detail.description}</p>
        {!loading && coverage.lastSuccessfulRunAt && (
          <p>
            Last successful backup:{' '}
            <time dateTime={coverage.lastSuccessfulRunAt} title={formatDateTime(coverage.lastSuccessfulRunAt, 'exact')}>
              {formatDateTime(coverage.lastSuccessfulRunAt)}
            </time>
          </p>
        )}
        {!loading && coverage.policies.length > 0 && <p>{coverage.policies.map((policy) => policy.name).join(', ')}</p>}
      </TooltipContent>
    </Tooltip>
  );
}
