import { HardDrive } from 'lucide-react';
import { VolumesTable } from './table';
import { RequiredComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { ActionBar } from '@/components/custom/action-bar';
import { VolumeDropdownActions, VolumeGroupActions } from './actions';
import VolumeForm from './form';

export const VolumeComponents: RequiredComponents = {
  Icon: <HardDrive className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return <VolumesTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: VolumeDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Volume" items={items} actions={Object.values(VolumeGroupActions)} />;
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

export const VolumeFormComponents: RequiredFormComponents = {
  Header: {
    Indicator: undefined,
    ActionButtons: undefined,
  },
  Form: ({ mode }) => {
    return <VolumeForm mode={mode} />;
  },
};
