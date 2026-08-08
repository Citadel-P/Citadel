import { createActionsBuilder } from '@/components/custom/actions-builder';
import { CircleCheck, CircleOff, Pause } from 'lucide-react';
import { useParams } from 'react-router';
import { SwarmNodeListView } from './hooks/useNodesGroup';

const asSelection = (resources: SwarmNodeListView | SwarmNodeListView[]) =>
  Array.isArray(resources) ? resources : [resources];

const canSetAvailability = (resources: SwarmNodeListView | SwarmNodeListView[], availability: string) => {
  const selected = asSelection(resources);
  return (
    selected.length > 0 &&
    selected.every((node) => !node.isStale) &&
    selected.some((node) => node.availability.toLowerCase() !== availability.toLowerCase())
  );
};

const nodeAvailabilityAction = (
  key: string,
  availability: 'Active' | 'Pause' | 'Drain',
  icon: typeof CircleCheck,
  confirm = false,
) => ({
  key,
  title: availability,
  type: 'command' as const,
  icon,
  mutateKey: 'updateSwarmNodesAvailability' as const,
  requiredCapabilities: ['canWrite'] as const,
  confirm,
  resourceType: 'Node' as const,
  argName: 'variables' as const,
  canExecute: (resources: SwarmNodeListView | SwarmNodeListView[]) => canSetAvailability(resources, availability),
  useVariables: (resources: SwarmNodeListView | SwarmNodeListView[]) => {
    const { platformId = '' } = useParams<{ platformId: string }>();
    return {
      platformId,
      data: {
        availability,
        nodes: asSelection(resources).map((node) => ({ nodeId: node.id, versionIndex: node.versionIndex })),
      },
    };
  },
});

const nodeActions = createActionsBuilder<SwarmNodeListView>()
  .addAction(nodeAvailabilityAction('active', 'Active', CircleCheck))
  .addAction(nodeAvailabilityAction('pause', 'Pause', Pause))
  .addAction(nodeAvailabilityAction('drain', 'Drain', CircleOff, true))
  .build();

export const NodeGroupActions = nodeActions.group;
export const NodeInfoActions = nodeActions.info;
