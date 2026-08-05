import type { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { ContainerExec } from '@/features/docker-resources/containers/container-info/container-exec';
import { useRead } from '@/lib/hooks';
import Loader from '@/components/ui/loader';
import type { SwarmTaskInfoView } from '../hooks/useTasksGroup';

export const TaskTerminal = ({ task }: { task: SwarmTaskInfoView }) => {
  const query = useRead(
    'getSwarmTaskTerminalTarget',
    { platformId: task.platformId, resourceId: task.id },
    { retry: false },
  );
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;

  if (query.isLoading) return <Loader />;

  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Unable to open terminal'} type="error">
        {problem.detail ?? 'Docker could not resolve the task container.'}
      </AlertMessage>
    );
  }

  const containerId = query.data?.data?.dockerContainerId;
  if (!containerId) {
    return (
      <AlertMessage title="Unable to open terminal" type="error">
        The running task container is not available.
      </AlertMessage>
    );
  }

  return <ContainerExec containerId={containerId} />;
};
