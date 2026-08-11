import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
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
  );
  const problem = (stats.error as { error?: ProblemDetails } | undefined)?.error;

  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Service stats unavailable'} type="info">
        {problem.detail ?? 'Citadel could not return stats for this Service.'}
      </AlertMessage>
    );
  }

  const latestStat = live.stats.at(-1) ?? stats.baseStats.at(-1);
  const missingNodes = stats.missingDockerNodeIds.join(', ');
  const observedTasks = Math.max(stats.observedTasks, live.observedContainers);
  const complete = stats.complete || (stats.expectedTasks > 0 && live.observedContainers >= stats.expectedTasks);

  return (
    <div className="flex flex-col gap-3">
      {!stats.isLoading && !complete && (
        <AlertMessage title="Partial Service statistics" type="warning">
          {`Showing ${observedTasks} of ${stats.expectedTasks} current Tasks${
            missingNodes ? `. Missing node data: ${missingNodes}.` : '.'
          }`}
        </AlertMessage>
      )}
      <ContainerStatsCharts
        resource={{ state: service.runningTaskCount > 0 ? 'Running' : 'Offline', containerStat: latestStat }}
        memory={stats}
        cpu={stats}
        network={stats}
        liveStats={live.stats}
      />
    </div>
  );
};
