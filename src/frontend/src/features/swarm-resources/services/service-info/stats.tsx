import { latestLiveStat } from '@/lib/stats-buckets';
import { statsBucketSeconds } from '@/lib/stats-range';
import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { StatsChartRange } from '@/components/custom/stats-chart-range';
import { ContainerStatsCharts } from '@/features/docker-resources/containers/container-info/container-stats';
import { SwarmServiceInfoView } from '../hooks/useServicesGroup';
import { useServiceStatsStream, useServiceStatsWindow } from '../hooks/useServiceStats';

type ServiceStatsResource = Pick<SwarmServiceInfoView, 'id' | 'platformId' | 'runningTaskCount'>;

export const ServiceStats = ({
  service,
  containerProjectionIds,
}: {
  service: ServiceStatsResource;
  containerProjectionIds?: string[];
}) => {
  const stats = useServiceStatsWindow(service.platformId, service.id);
  const live = useServiceStatsStream(
    service.platformId,
    containerProjectionIds ?? stats.observedContainerProjectionIds,
    statsBucketSeconds(stats.range.bounds),
    stats.range.hours * 3600,
  );
  const problem = (stats.error as { error?: ProblemDetails } | undefined)?.error;

  if (problem) {
    return (
      <div className="flex flex-col gap-3">
        <StatsChartRange range={stats.range}>
          <AlertMessage title={problem.title ?? 'Service stats unavailable'} type="info">
            {problem.detail ?? 'Citadel could not return stats for this Service.'}
          </AlertMessage>
        </StatsChartRange>
      </div>
    );
  }

  const latestStat = latestLiveStat(live.stats) ?? stats.baseStats.at(-1);
  const missingNodes = stats.missingDockerNodeIds.join(', ');
  const observedTasks = Math.max(stats.observedTasks, live.observedContainers);
  const complete = stats.complete || (stats.expectedTasks > 0 && live.observedContainers >= stats.expectedTasks);

  return (
    <div className="flex flex-col gap-3">
      {stats.range.live && !stats.isLoading && !complete && (
        <AlertMessage title="Partial Service statistics" type="warning">
          {`Showing ${observedTasks} of ${stats.expectedTasks} current Tasks${
            missingNodes ? `. Missing node data: ${missingNodes}.` : '.'
          }`}
        </AlertMessage>
      )}
      <ContainerStatsCharts
        resource={{ state: Number(service.runningTaskCount) > 0 ? 'Running' : 'Offline', containerStat: latestStat }}
        query={stats}
        liveStats={live.stats}
      />
    </div>
  );
};
