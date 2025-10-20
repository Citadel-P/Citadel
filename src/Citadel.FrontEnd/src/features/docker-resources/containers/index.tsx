import { Box } from 'lucide-react';
import { ContainersTable } from './table';
import { ActionBar } from './action-bar';
import { DeleteDialog } from './delete-dialog';
import { RequiredDockerComponents, ResourceDataHookResult } from '@/pages/types';
import { useContainersGroup } from './hooks/useContainersGroup';

export const ContainerComponents: RequiredDockerComponents = {
  Icon: <Box className="h-4 w-4" />,
  Table: ({ items, isLoading }) => {
    return <ContainersTable items={items} isLoading={isLoading} />;
  },
  ActionBar: ({ items }) => {
    return <ActionBar items={items} />;
  },
  DeleteDialog: () => {
    return <DeleteDialog />;
  },
  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { containersInfo, isLoading } = useContainersGroup(platformId);
    return { items: containersInfo?.containers ?? [], isLoading };
  },
  header: {
    showAdd: false,
    showSearch: true,
  },
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
