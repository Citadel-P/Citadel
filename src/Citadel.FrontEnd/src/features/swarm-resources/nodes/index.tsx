import { RegularResourceComponents } from '@/pages/types';
import { Network } from 'lucide-react';
import { SwarmNodeListView, useNodesGroup } from './hooks/useNodesGroup';
import { NodesTable } from './table';
import { NodeEditDropdownAction } from './node-edit-dialog';
import { ActionBar } from '@/components/custom/action-bar';
import { NodeGroupActions } from './actions';

export const NodeComponents: RegularResourceComponents<SwarmNodeListView> = {
  Icon: Network,
  header: {
    title: 'Nodes',
    subtitle: 'Docker Swarm node inventory and task placement.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, actions, isLoading }) => <NodesTable items={items} actions={actions} isLoading={isLoading} />,
  DropdownActions: { edit: NodeEditDropdownAction },
  GroupActions: ({ items }) => <ActionBar type="Node" items={items} actions={Object.values(NodeGroupActions)} />,
  useData: (platformId) => {
    const { items, isLoading, error, refetch, isFetching } = useNodesGroup(platformId);
    return { error, refetch, isFetching, items, isLoading, capabilities: undefined };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter(
          (item) =>
            item.hostname?.toLowerCase().includes(value) ||
            item.id?.toLowerCase().includes(value) ||
            item.address?.toLowerCase().includes(value),
        )
      : items;
  },
};
