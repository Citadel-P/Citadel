import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { NoResourceActions, StaleBadge, StaleWarning, SwarmLogs } from '../../shared';
import { SwarmTaskInfoView, useTaskInfoGroup } from '../hooks/useTasksGroup';
import { TaskInspect } from './inspect';
import { TaskStats } from './stats';
import { TaskInfoTable } from './table';
import { TaskTerminal } from './terminal';

const canOpenTaskTerminal = (resource: SwarmTaskInfoView) =>
  resource.state.toLowerCase() === 'running' && resource.capabilities?.canOpenTerminal === true;

export const TaskInfoComponents: RequiredSwarmInfoComponents<SwarmTaskInfoView> = {
  Header: {
    Indicator: ({ resource }) => <StateIndicator value={resource.state} kind="swarmTask" />,
    NameSuffix: StaleBadge,
    ActionButtons: NoResourceActions,
  },
  SubHeader: ({ resource }) => (
    <div className="flex flex-col gap-3">
      <StaleWarning resource={resource} />
      <TaskInfoTable task={resource} />
    </div>
  ),
  Tabs: [
    {
      label: 'Logs',
      disabled: (resource) => resource.capabilities?.canViewLogs !== true,
      Content: ({ resource }) => (
        <SwarmLogs
          platformId={resource.platformId}
          resourceId={resource.id}
          resource="task"
          capabilities={resource.capabilities}
        />
      ),
    },
    {
      label: 'Inspect',
      disabled: (resource) => resource.capabilities?.canInspect !== true,
      Content: ({ resource }) => <TaskInspect task={resource} />,
    },
    {
      label: 'Terminal',
      disabled: (resource) => !canOpenTaskTerminal(resource),
      Content: ({ resource }) => <TaskTerminal task={resource} />,
    },
    {
      label: 'Stats',
      disabled: (resource) => resource.capabilities?.canRead !== true,
      Content: ({ resource }) => <TaskStats task={resource} />,
    },
  ],
  useData: useTaskInfoGroup,
};
