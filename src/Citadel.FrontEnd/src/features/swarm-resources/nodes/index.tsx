import { RegularResourceComponents } from '@/pages/types';
import { Network } from 'lucide-react';
import { SwarmNodeListView, useNodesGroup } from './hooks/useNodesGroup';
import { NodesTable } from './table';

export const NodeComponents: RegularResourceComponents<SwarmNodeListView> = {
  Icon: Network,
  header: {
    title: 'Nodes',
    subtitle: 'Docker Swarm node inventory and task placement.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, isLoading }) => <NodesTable items={items} isLoading={isLoading} />,
  useData: (platformId) => {
    const { items, isLoading } = useNodesGroup(platformId);
    return { items, isLoading, capabilities: undefined };
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
