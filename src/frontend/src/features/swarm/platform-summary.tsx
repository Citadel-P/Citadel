import { PlatformBackupSummary, WorkloadStatusCounts, ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import Loader from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import { useMutate } from '@/lib/hooks';
import { PlatformResourceMetric } from '@/features/platforms/forms/platform-stats';
import { applySwarmInventoryToOverview, useSwarmOverview } from './hooks/useSwarmOverview';
import {
  getBackupMetric,
  getBackupSourceStates,
  PLATFORM_BACKUP_ICON_CLASS_NAME,
} from '@/features/platforms/platform-backups';
import { ArchiveRestore, Boxes, CircleDot, ListTodo, Network, RefreshCw, Server } from 'lucide-react';
import { SwarmQuorumStatus } from './swarm-quorum-status';
import { getServiceStates } from '@/features/platforms/platform-workload-status';

export { applySwarmInventoryToOverview };

export const SwarmPlatformSummary = ({
  platformId,
  networkCount,
  serviceStatusCounts,
  backupSummary,
  isBackupSummaryLoading = false,
  isBackupSummaryError = false,
}: {
  platformId: string;
  networkCount: number | string;
  serviceStatusCounts: WorkloadStatusCounts;
  backupSummary?: PlatformBackupSummary;
  isBackupSummaryLoading?: boolean;
  isBackupSummaryError?: boolean;
}) => {
  const { overview, isLoading, error, refetch } = useSwarmOverview(platformId);
  const refresh = useMutate('refreshSwarmInventory', {
    onSuccess: async () => {
      await refetch();
    },
  });
  const refreshProblem = (refresh.error as unknown as { error?: ProblemDetails } | undefined)?.error;
  const problem = (error as unknown as { error?: ProblemDetails } | undefined)?.error;
  const backupMetric = getBackupMetric(backupSummary, isBackupSummaryLoading, isBackupSummaryError);

  return (
    <section className="space-y-3 pb-3" aria-label="Swarm cluster summary">
      {isLoading && !overview && <Loader />}
      {problem && (
        <AlertMessage title={problem.title ?? 'Unable to load the Swarm'} type="error">
          {problem.detail ?? 'The Swarm summary could not be loaded.'}
        </AlertMessage>
      )}
      {overview?.message && (
        <AlertMessage
          title={overview.health}
          type={overview.health === 'Offline' ? 'error' : 'warning'}
          action={
            overview.isStale && overview.capabilities.canRead ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={refresh.isPending}
                onClick={() => refresh.mutate({ platformId })}>
                <RefreshCw className={refresh.isPending ? 'animate-spin' : undefined} aria-hidden="true" />
                {refresh.isPending ? 'Refreshing…' : 'Refresh inventory'}
              </Button>
            ) : undefined
          }>
          {overview.message}
        </AlertMessage>
      )}
      {refresh.isSuccess && overview?.isStale && (
        <AlertMessage title="Inventory refreshed" type="info">
          The manager inventory is up to date. Check Node Agent connections for remaining stale node-local inventory.
        </AlertMessage>
      )}
      {refresh.isError && overview?.isStale && (
        <AlertMessage title="Inventory refresh failed" type="error">
          {refreshProblem?.detail ?? 'Unable to refresh inventory. Check the Swarm manager connection and try again.'}
        </AlertMessage>
      )}
      <div className="grid gap-px overflow-hidden rounded-md border bg-border sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        <PlatformResourceMetric
          icon={CircleDot}
          iconClassName="text-emerald-500"
          label="Managers"
          value={overview?.managerCount ?? '-'}
          to={`/platforms/${platformId}/nodes`}
          detail={
            overview?.quorum ? (
              <SwarmQuorumStatus quorum={overview.quorum} managerCount={overview.managerCount} showCounts={false} />
            ) : undefined
          }
        />
        <PlatformResourceMetric
          icon={Server}
          iconClassName="text-cyan-500"
          label="Nodes"
          value={overview?.nodeCount ?? '-'}
          to={`/platforms/${platformId}/nodes`}
        />
        <PlatformResourceMetric
          icon={Boxes}
          iconClassName="text-violet-500"
          label="Services"
          value={serviceStatusCounts.total}
          to={`/swarm-services?platformId=${platformId}`}
          states={getServiceStates(serviceStatusCounts)}
        />
        <PlatformResourceMetric
          icon={ListTodo}
          iconClassName="text-sky-500"
          label="Running tasks"
          value={overview ? `${overview.runningTaskCount}/${overview.desiredTaskCount}` : '-'}
          to={`/platforms/${platformId}/tasks`}
        />
        <PlatformResourceMetric
          icon={Network}
          iconClassName="text-cyan-500"
          label="Networks"
          value={overview?.networkCount ?? networkCount}
          to={`/platforms/${platformId}/networks`}
        />
        <PlatformResourceMetric
          icon={ArchiveRestore}
          iconClassName={PLATFORM_BACKUP_ICON_CLASS_NAME}
          label="Backups"
          value={backupMetric.value}
          to="/backup-policies"
          states={getBackupSourceStates(backupSummary)}
          detail={backupMetric.detail}
          detailTitle={backupMetric.detailTitle}
        />
      </div>
    </section>
  );
};
