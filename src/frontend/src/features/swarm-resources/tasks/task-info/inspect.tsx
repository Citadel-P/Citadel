import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { useRead } from '@/lib/hooks';
import { InspectionPanel } from '@/components/custom/inspection-panel';
import { SwarmTaskInfoView } from '../hooks/useTasksGroup';
import { RuntimeProcesses } from '@/features/docker-resources/containers/container-info/runtime-processes';

export const TaskInspect = ({ task }: { task: SwarmTaskInfoView }) => {
  const query = useRead('inspectSwarmTask', { platformId: task.platformId, resourceId: task.id });
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Unable to inspect task'} type="error">
        {problem.detail ?? 'Docker could not inspect this task.'}
      </AlertMessage>
    );
  }

  return (
    <InspectionPanel
      data={query.data?.data}
      isLoading={query.isLoading}
      filename={`inspect-task-${task.id}.json`}
      loadingMessage="// Loading task inspection data..."
      noDataMessage="// No task inspection data available">
      <RuntimeProcesses
        key={`${task.id}:${task.nodeId}`}
        platformId={task.platformId}
        taskId={task.id}
        dockerNodeId={task.nodeId}
      />
    </InspectionPanel>
  );
};
