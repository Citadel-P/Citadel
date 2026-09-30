import { DockerLabelsSection } from '@/components/custom/common';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { getSwarmNodeIndicatorValue, StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { Detail, StaleBadge, StaleWarning } from '../../shared';
import { TasksTable } from '../../tasks/table';
import { SwarmNodeInfoView, useNodeInfoGroup } from '../hooks/useNodesGroup';
import { NodeInspect } from './inspect';
import { NodeEditInfoAction } from '../node-edit-dialog';
import { NodeInfoActions } from '../actions';

const NodeActionButtons = ({ resource }: { resource: SwarmNodeInfoView }) => (
  <GenericActionBarButtons
    resource={resource}
    actions={Object.values(NodeInfoActions)}
    standaloneActions={[NodeEditInfoAction]}
  />
);

export const NodeInfoComponents: RequiredSwarmInfoComponents<SwarmNodeInfoView> = {
  Header: {
    Indicator: ({ resource }) => (
      <StateIndicator variant="badge" value={getSwarmNodeIndicatorValue(resource)} kind="swarmNode" />
    ),
    NameSuffix: StaleBadge,
    ActionButtons: NodeActionButtons,
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
  Footer: ({ resource }) => <DockerLabelsSection labels={resource.labels} />,
  Tabs: [
    {
      label: 'Inspect',
      disabled: (resource) => resource.capabilities?.canInspect !== true,
      Content: ({ resource }) => <NodeInspect node={resource} />,
    },
  ],
  useData: useNodeInfoGroup,
};
