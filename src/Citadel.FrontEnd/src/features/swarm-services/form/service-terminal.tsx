import { useMemo, useState } from 'react';
import type { SwarmTaskView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import Loader from '@/components/ui/loader';
import { TerminalTargetSelect } from '@/features/docker-resources/containers/container-info/container-exec';
import { TaskTerminal } from '@/features/swarm-resources/tasks/task-info/terminal';
import { useRead } from '@/lib/hooks';
import { getTaskName } from '@/lib/utils';

type ServiceTerminalProps = {
  platformId: string;
  tasks: SwarmTaskView[];
  tasksLoading: boolean;
};

export const ServiceTerminal = ({ platformId, tasks, tasksLoading }: ServiceTerminalProps) => {
  const [selectedTaskId, setSelectedTaskId] = useState<string>();
  const platformQuery = useRead('getPlatfom', { id: platformId });
  const platform = platformQuery.data?.data;
  const descriptor = platform?.platformDescriptor;
  const connectedNodeId = descriptor && 'nodeID' in descriptor ? descriptor.nodeID : undefined;
  const runningTasks = useMemo(() => tasks.filter((task) => task.state.toLowerCase() === 'running'), [tasks]);
  const canOpenTerminal = (task: SwarmTaskView) =>
    (task.capabilities ?? platform?.capabilities)?.canOpenTerminal === true;
  const localTasks = runningTasks.filter((task) => task.nodeId === connectedNodeId && canOpenTerminal(task));
  const selectedTask = localTasks.find((task) => task.id === selectedTaskId) ?? localTasks[0];
  const hasRemoteTasks = runningTasks.some((task) => task.nodeId !== connectedNodeId);

  if (tasksLoading || platformQuery.isLoading) return <Loader />;

  if (runningTasks.length === 0) {
    return (
      <AlertMessage title="No running Tasks" type="info">
        A terminal is available after Docker starts at least one Task.
      </AlertMessage>
    );
  }

  if (!connectedNodeId) {
    return (
      <AlertMessage title="Connected manager unavailable" type="error">
        Citadel could not identify the Swarm manager used by this Platform.
      </AlertMessage>
    );
  }

  const hasTerminalPermission = runningTasks.some(canOpenTerminal);
  const taskSelect = (
    <TerminalTargetSelect
      value={selectedTask?.id}
      options={runningTasks.map((task) => {
        const isLocal = task.nodeId === connectedNodeId;
        const permitted = canOpenTerminal(task);
        const suffix = !permitted ? 'permission required' : !isLocal ? 'remote node' : 'connected manager';
        return {
          value: task.id,
          label: `${getTaskName(task)} — ${task.nodeHostname || task.nodeId.slice(0, 12)} (${suffix})`,
          disabled: !isLocal || !permitted,
        };
      })}
      placeholder="Select a running Task"
      ariaLabel="Task"
      onValueChange={setSelectedTaskId}
    />
  );

  return (
    <div className="flex min-w-0 flex-col gap-3">
      {!selectedTask && taskSelect}
      {!hasTerminalPermission && (
        <AlertMessage title="Terminal permission required" type="warning">
          Terminal access requires Platform Read and Terminal permission.
        </AlertMessage>
      )}
      {hasTerminalPermission && !selectedTask && (
        <AlertMessage title="No local Task available" type="info">
          Docker can open a terminal only on the node running the Task. None of this Service&apos;s running Tasks are on
          the connected manager.
        </AlertMessage>
      )}
      {selectedTask && hasRemoteTasks && (
        <p className="text-xs text-muted-foreground">
          Terminal access is unavailable for worker-node Tasks because Docker exec is node-local.
        </p>
      )}
      {selectedTask && (
        <TaskTerminal key={selectedTask.id} task={{ ...selectedTask, platformId }} toolbarStart={taskSelect} />
      )}
    </div>
  );
};
