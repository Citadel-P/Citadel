import { PlatformBackupSummaryView, ProblemDetails, SwarmOverviewView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import Loader from '@/components/ui/loader';
import { PlatformResourceMetric } from '@/features/platforms/forms/platform-stats';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import {
  getBackupMetric,
  getBackupSourceStates,
  PLATFORM_BACKUP_ICON_CLASS_NAME,
} from '@/features/platforms/platform-backups';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { ArchiveRestore, Boxes, CircleDot, ListTodo, Network, Server } from 'lucide-react';
import { useCallback } from 'react';

type CachedResponse<T> = { data: T };

const count = (value: number | string) => Number(value) || 0;

export const applySwarmInventoryToOverview = (
  inventory: SwarmInventoryUpdate,
  _previous?: SwarmOverviewView,
): SwarmOverviewView => {
  const isStale =
    inventory.nodes.items.some((item) => item.isStale) ||
    inventory.services.items.some((item) => item.isStale) ||
    inventory.tasks.items.some((item) => item.isStale) ||
    inventory.networks.items.some((item) => item.isStale) ||
    inventory.secrets.items.some((item) => item.isStale) ||
    inventory.configs.items.some((item) => item.isStale);
  const health = isStale ? 'Stale' : 'Healthy';
  const message = isStale ? 'The latest inventory refresh failed. Last-known inventory may be out of date.' : null;

  return {
    platformId: inventory.platformId,
    health,
    message,
    isStale,
    nodeCount: inventory.nodes.items.length,
    managerCount: inventory.nodes.items.filter((node) => node.role.toLowerCase() === 'manager').length,
    serviceCount: inventory.services.items.length,
    runningTaskCount: inventory.services.items.reduce((total, service) => total + count(service.runningTaskCount), 0),
    desiredTaskCount: inventory.services.items.reduce((total, service) => total + count(service.desiredTaskCount), 0),
    networkCount: inventory.networks.items.length,
  };
};

export const SwarmPlatformSummary = ({
  platformId,
  networkCount,
  backupSummary,
  isBackupSummaryLoading = false,
  isBackupSummaryError = false,
}: {
  platformId: string;
  networkCount: number | string;
  backupSummary?: PlatformBackupSummaryView;
  isBackupSummaryLoading?: boolean;
  isBackupSummaryError?: boolean;
}) => {
  const query = useRead('getSwarmOverview', { platformId });
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (inventory.platformId !== platformId) return;
      const queryKey = ['getSwarmOverview', { platformId }] as const;
      queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, (previous) => ({
        ...previous,
        data: applySwarmInventoryToOverview(inventory, previous?.data),
      }));
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
    },
    [platformId, queryClient],
  );
  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });

  const overview = query.data?.data;
  const problem = (query.error as unknown as { error?: ProblemDetails } | undefined)?.error;
  const backupMetric = getBackupMetric(backupSummary, isBackupSummaryLoading, isBackupSummaryError);

  return (
    <section className="space-y-3 pb-3" aria-label="Swarm cluster summary">
      {query.isLoading && !overview && <Loader />}
      {problem && (
        <AlertMessage title={problem.title ?? 'Unable to load the Swarm'} type="error">
          {problem.detail ?? 'The Swarm summary could not be loaded.'}
        </AlertMessage>
      )}
      {overview?.message && (
        <AlertMessage title={overview.health} type={overview.health === 'Offline' ? 'error' : 'warning'}>
          {overview.message}
        </AlertMessage>
      )}
      <div className="grid gap-px overflow-hidden rounded-md border bg-border sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        <PlatformResourceMetric
          icon={CircleDot}
          iconClassName="text-emerald-500"
          label="Managers"
          value={overview?.managerCount ?? '-'}
          to={`/platforms/${platformId}/nodes`}
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
          value={overview?.serviceCount ?? '-'}
          to={`/platforms/${platformId}/services`}
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
          value={networkCount}
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
