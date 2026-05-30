import { Box } from 'lucide-react';
import { ContainersTable } from './table';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useContainersGroup } from './hooks/useContainersGroup';
import { ContainerDropdownActions, ContainerGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';

export const ContainerComponents: RequiredComponents = {
  Icon: Box,
  Content: ({ items, isLoading, actions }) => {
    return <ContainersTable items={items} isLoading={isLoading} actions={actions} />;
  },

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { containersInfo, isLoading } = useContainersGroup(platformId);
    return { items: containersInfo?.containers ?? [], isLoading, capabilities: undefined };
  },
  header: {
    showAdd: false,
    showSearch: true,
  },
  DropdownActions: ContainerDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="Container" items={items} actions={Object.values(ContainerGroupActions)} />
  ),

  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (c) =>
        c.name?.toLowerCase().includes(s) ||
        c.containerId?.toLowerCase().includes(s) ||
        c.containerId?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};
