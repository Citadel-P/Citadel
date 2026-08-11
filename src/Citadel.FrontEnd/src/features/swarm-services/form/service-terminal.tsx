import { useMemo, useState } from 'react';
import type { SwarmTaskView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import Loader from '@/components/ui/loader';
import { TerminalTargetSelect } from '@/features/docker-resources/containers/container-info/container-exec';
import { TaskTerminal } from '@/features/swarm-resources/tasks/task-info/terminal';
import { getTaskName } from '@/lib/utils';

type ServiceTerminalProps = {
  platformId: string;
  tasks: SwarmTaskView[];
  tasksLoading: boolean;
};

export const ServiceTerminal = ({ platformId, tasks, tasksLoading }: ServiceTerminalProps) => {
  const [selectedTaskId, setSelectedTaskId] = useState<string>();
  const runningTasks = useMemo(() => tasks.filter((task) => task.state.toLowerCase() === 'running'), [tasks]);
  const canOpenTerminal = (task: SwarmTaskView) => task.capabilities?.canOpenTerminal === true;
  const availableTasks = runningTasks.filter(canOpenTerminal);
  const selectedTask = availableTasks.find((task) => task.id === selectedTaskId) ?? availableTasks[0];

  if (tasksLoading) return <Loader />;

  if (runningTasks.length === 0) {
    return (
      <AlertMessage title="No running Tasks" type="info">
        A terminal is available after Docker starts at least one Task.
      </AlertMessage>
    );
  }

  const hasTerminalPermission = runningTasks.some(canOpenTerminal);
  const taskSelect = (
    <TerminalTargetSelect
      value={selectedTask?.id}
      options={runningTasks.map((task) => {
        const permitted = canOpenTerminal(task);
        const suffix = permitted ? task.nodeHostname || task.nodeId.slice(0, 12) : 'permission required';
        return {
          value: task.id,
          label: `${getTaskName(task)} — ${suffix}`,
          disabled: !permitted,
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
      {selectedTask && (
        <TaskTerminal key={selectedTask.id} task={{ ...selectedTask, platformId }} toolbarStart={taskSelect} />
      )}
    </div>
  );
};
