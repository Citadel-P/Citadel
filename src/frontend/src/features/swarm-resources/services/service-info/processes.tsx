import { useState } from 'react';
import { ProcessesSection } from '@/features/docker-resources/containers/container-info/container-processes';
import { RuntimeProcessList } from '@/features/docker-resources/containers/container-info/runtime-processes';
import { RuntimeTargetSelect } from '@/components/custom/runtime-target-select';
import { useServiceTasksGroup } from '../../tasks/hooks/useTasksGroup';
import { getTaskName } from '@/lib/utils';

type ServiceProcessesProps = { platformId: string; serviceId: string };

function ServiceProcessList({ platformId, serviceId }: ServiceProcessesProps) {
  const { items, isLoading, error } = useServiceTasksGroup(platformId, serviceId);
  const [selectedId, setSelectedId] = useState<string>();
  const tasks = items.filter(
    (task) => task.serviceId === serviceId && task.state.toLowerCase() === 'running' && !task.isStale,
  );
  const permitted = tasks.filter((task) => task.capabilities?.canInspect === true);
  const selected = permitted.find((task) => task.id === selectedId) ?? permitted[0];
  const messageClass = 'border-t px-4 py-8 text-center text-sm text-muted-foreground';
  if (isLoading)
    return (
      <p role="status" className={messageClass}>
        Loading tasks…
      </p>
    );
  if (error)
    return (
      <p role="alert" className={messageClass}>
        Unable to load tasks. Processes require Platform Read and Inspect permission.
      </p>
    );
  if (!tasks.length) return <p className={messageClass}>No running tasks available.</p>;
  if (!selected) return <p className={messageClass}>Processes require Platform Read and Inspect permission.</p>;
  return (
    <div>
      <div className="border-t px-4 py-3">
        <RuntimeTargetSelect
          value={selected.id}
          options={tasks.map((task) => ({
            value: task.id,
            label: `${getTaskName(task)} — ${task.nodeHostname || task.nodeId.slice(0, 12)}`,
            disabled: task.capabilities?.canInspect !== true,
          }))}
          placeholder="Select a running task"
          ariaLabel="Processes task"
          onValueChange={setSelectedId}
        />
      </div>
      <RuntimeProcessList
        key={`${selected.id}:${selected.nodeId}`}
        platformId={platformId}
        taskId={selected.id}
        dockerNodeId={selected.nodeId}
      />
    </div>
  );
}

export function ServiceProcesses(props: ServiceProcessesProps) {
  return (
    <ProcessesSection>
      <ServiceProcessList key={`${props.platformId}:${props.serviceId}`} {...props} />
    </ProcessesSection>
  );
}
