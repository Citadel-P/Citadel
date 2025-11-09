import { Network } from 'lucide-react';
import { NetworksTable } from './table';
import { RequiredDockerComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { NetworkDropdownActions, NetworkGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';

export const NetworkComponents: RequiredDockerComponents = {
  Icon: <Network className="h-4 w-4" />,
  Table: ({ items, actions, isLoading }) => {
    return <NetworksTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: NetworkDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Network" items={items} actions={Object.values(NetworkGroupActions)} />;
  },
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { data, isLoading } = useRead(`listNetworks`, { platformId: platformId });
    return { items: data?.data?.networks ?? [], isLoading };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (v) =>
        v.name?.toLowerCase().includes(s) ||
        v.id?.toLowerCase().includes(s) ||
        v.id?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};
