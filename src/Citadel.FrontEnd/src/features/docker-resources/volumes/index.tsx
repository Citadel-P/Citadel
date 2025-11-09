import { HardDrive } from 'lucide-react';
import { VolumesTable } from './table';
import { RequiredDockerComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { ActionBar2 } from '@/components/custom/action-bar';
import { VolumeDropdownActions, VolumeGroupActions } from './actions';

export const VolumeComponents: RequiredDockerComponents = {
  Icon: <HardDrive className="h-4 w-4" />,
  Table: ({ items, actions, isLoading }) => {
    return <VolumesTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: VolumeDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar2 type="Volume" items={items} actions={Object.values(VolumeGroupActions)} />;
  },

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { data, isLoading } = useRead(`listVolumes`, { platformId: platformId });
    return { items: data?.data?.volumes ?? [], isLoading };
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
