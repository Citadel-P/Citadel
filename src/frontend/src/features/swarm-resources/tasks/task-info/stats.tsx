import { latestLiveStat } from '@/lib/stats-buckets';
import { statsBucketSeconds } from '@/lib/stats-range';
import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { StatsChartRange } from '@/components/custom/stats-chart-range';
import { ContainerStatsCharts } from '@/features/docker-resources/containers/container-info/container-stats';
import { SwarmTaskInfoView } from '../hooks/useTasksGroup';
import { useTaskStatsStream, useTaskStatsWindow } from '../hooks/useTaskStats';

export const TaskStats = ({ task }: { task: SwarmTaskInfoView }) => {
  const stats = useTaskStatsWindow(task.platformId, task.id);
  const containerProjectionId = stats.containerProjectionId;
  const liveStats = useTaskStatsStream(
    task.platformId,
    containerProjectionId,
    statsBucketSeconds(stats.range.bounds),
    stats.range.hours * 3600,
  );
  const problem = (stats.error as { error?: ProblemDetails } | undefined)?.error;

  if (problem) {
    return (
      <div className="flex flex-col gap-3">
        <StatsChartRange range={stats.range}>
          <AlertMessage title={problem.title ?? 'Task stats unavailable'} type="info">
            {problem.detail ?? 'Citadel could not return stats for this task.'}
          </AlertMessage>
        </StatsChartRange>
      </div>
    );
  }

  const latestStat = latestLiveStat(liveStats) ?? stats.baseStats.at(-1);
  return (
    <ContainerStatsCharts
      resource={{ state: task.state, containerStat: latestStat }}
      query={stats}
      liveStats={liveStats}
    />
  );
};
