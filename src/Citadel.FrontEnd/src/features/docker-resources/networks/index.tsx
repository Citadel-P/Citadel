import { Network } from 'lucide-react';
import { NetworksTable } from './table';
import { RequiredComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { NetworkDropdownActions, NetworkGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import NetworkForm from './form';
import { useNetworksGroup } from './hooks/useNetworksGroup';

export const NetworkComponents: RequiredComponents = {
  Icon: <Network className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return <NetworksTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: NetworkDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Network" items={items} actions={Object.values(NetworkGroupActions)} />;
  },
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { networks, isLoading } = useNetworksGroup(platformId);
    return { items: networks?.networks ?? [], isLoading };
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

export const NetworkFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => <NetworkForm mode="add" />,
  },
  EditForm: {},
};
