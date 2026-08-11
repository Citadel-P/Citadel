import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { ContainerStatsCharts } from '@/features/docker-resources/containers/container-info/container-stats';
import { SwarmTaskInfoView } from '../hooks/useTasksGroup';
import { useTaskStatsStream, useTaskStatsWindow } from '../hooks/useTaskStats';

export const TaskStats = ({ task }: { task: SwarmTaskInfoView }) => {
  const memory = useTaskStatsWindow(task.platformId, task.id);
  const cpu = useTaskStatsWindow(task.platformId, task.id);
  const network = useTaskStatsWindow(task.platformId, task.id);
  const containerProjectionId =
    memory.containerProjectionId ?? cpu.containerProjectionId ?? network.containerProjectionId;
  const liveStats = useTaskStatsStream(task.platformId, containerProjectionId);
  const problem = ((memory.error ?? cpu.error ?? network.error) as { error?: ProblemDetails } | undefined)?.error;

  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Task stats unavailable'} type="info">
        {problem.detail ?? 'Citadel could not return stats for this task.'}
      </AlertMessage>
    );
  }

  const latestStat = liveStats.at(-1) ?? memory.baseStats.at(-1);
  return (
    <ContainerStatsCharts
      resource={{ state: task.state, containerStat: latestStat }}
      memory={memory}
      cpu={cpu}
      network={network}
      liveStats={liveStats}
    />
  );
};
