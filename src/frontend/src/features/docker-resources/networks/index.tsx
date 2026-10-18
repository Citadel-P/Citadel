import { Network } from 'lucide-react';
import { NetworkViews, NetworkViewControls } from './views';
import { RequiredComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { NetworkDropdownActions, NetworkGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import NetworkForm from './form';
import { useNetworksGroup } from './hooks/useNetworksGroup';
import { networkFilterParams, useNetworkFilters } from './filters';
import { filterNetworks } from './network-presentation';

const EMPTY_NETWORKS: never[] = [];

export const NetworkComponents: RequiredComponents = {
  Icon: Network,
  Content: ({ items, actions, isLoading, isFiltered }) => {
    return <NetworkViews items={items} actions={actions} isLoading={isLoading} isFiltered={isFiltered} />;
  },
  DropdownActions: NetworkDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Network" items={items} actions={Object.values(NetworkGroupActions)} />,
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { networks, isLoading, capabilities, error, refetch, isFetching } = useNetworksGroup(platformId);
    const items = useNetworkFilters(networks?.networks ?? EMPTY_NETWORKS);
    return { error, refetch, isFetching, items, isLoading, capabilities };
  },
  header: { showSearch: true, showAdd: true, activeFilterParams: networkFilterParams, Extra: NetworkViewControls },
  filterItems: filterNetworks,
};

export const NetworkFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => <NetworkForm mode="add" />,
  },
  EditForm: undefined,
};
