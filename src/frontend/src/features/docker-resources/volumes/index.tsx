import { HardDrive } from 'lucide-react';
import { VolumesTable } from './table';
import { RequiredComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { VolumeDropdownActions, VolumeGroupActions } from './actions';
import { useVolumesGroup } from './hooks/useVolumesGroup';
import VolumeForm from './form';

const EMPTY_VOLUMES: never[] = [];

export const VolumeComponents: RequiredComponents = {
  Icon: HardDrive,
  Content: ({ items, actions, isLoading }) => {
    return <VolumesTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: VolumeDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Volume" items={items} actions={Object.values(VolumeGroupActions)} />;
  },

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { volumes, capabilities, isLoading, error, refetch, isFetching } = useVolumesGroup(platformId);
    return { error, refetch, isFetching, items: volumes?.volumes ?? EMPTY_VOLUMES, isLoading, capabilities };
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
  AddForm: {
    Content: () => <VolumeForm mode="add" />,
  },
  EditForm: undefined,
};
