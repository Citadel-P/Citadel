import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { Detail, NoResourceActions, StaleBadge, StaleWarning } from '../../shared';
import { TasksTable } from '../../tasks/table';
import { SwarmNodeInfoView, useNodeInfoGroup } from '../hooks/useNodesGroup';
import { NodeInspect } from './inspect';

export const NodeInfoComponents: RequiredSwarmInfoComponents<SwarmNodeInfoView> = {
  Header: {
    Indicator: ({ resource }) => (
      <StateIndicator value={resource.isStale ? 'unknown' : resource.status} kind="swarmNode" />
    ),
    NameSuffix: StaleBadge,
    ActionButtons: NoResourceActions,
  },
  SubHeader: ({ resource }) => (
    <div className="flex flex-col gap-3">
      <StaleWarning resource={resource} />
      <div className="grid grid-cols-2 gap-x-6 gap-y-2 border-b border-border/60 pb-3 sm:grid-cols-4">
        <Detail label="Role" value={resource.role} />
        <Detail label="Availability" value={resource.availability} />
        <Detail label="Engine" value={resource.engineVersion || '-'} />
        <Detail label="Address" value={resource.address || '-'} />
      </div>
      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold text-foreground">Tasks</h2>
        <TasksTable items={resource.tasks} isLoading={false} showNode={false} />
      </div>
    </div>
  ),
  Tabs: [
    {
      label: 'Inspect',
      disabled: (resource) => resource.capabilities?.canInspect !== true,
      Content: ({ resource }) => <NodeInspect node={resource} />,
    },
  ],
  useData: useNodeInfoGroup,
};
