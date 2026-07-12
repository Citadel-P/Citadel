import { Network } from 'lucide-react';
import { NetworksTable } from './table';
import { RequiredComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { NetworkDropdownActions, NetworkGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import NetworkForm from './form';
import { useNetworksGroup } from './hooks/useNetworksGroup';

const EMPTY_NETWORKS: never[] = [];

export const NetworkComponents: RequiredComponents = {
  Icon: Network,
  Content: ({ items, actions, isLoading }) => {
    return <NetworksTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: NetworkDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Network" items={items} actions={Object.values(NetworkGroupActions)} />;
  },
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { networks, isLoading, capabilities } = useNetworksGroup(platformId);
    return { items: networks?.networks ?? EMPTY_NETWORKS, isLoading, capabilities };
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
  EditForm: undefined,
};
