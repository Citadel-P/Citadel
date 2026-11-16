import { HardDrive } from 'lucide-react';
import { VolumesTable } from './table';
import { RegularResourceComponents, RequiredFormComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { VolumeDropdownActions, VolumeGroupActions } from './actions';
import { useVolumesGroup } from './hooks/useVolumesGroup';
import VolumeForm from './form';
import { BackupStatusFilter, useBackupStatusFilter } from '@/features/backup-policies/table-coverage';

const EMPTY_VOLUMES: never[] = [];

export const VolumeComponents: RegularResourceComponents = {
  Icon: HardDrive,
  Content: VolumesTable,
  header: {
    showSearch: true,
    showAdd: true,
    activeFilterParams: ['backupStatus'],
    Extra: BackupStatusFilter,
  },
  DropdownActions: VolumeDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Volume" items={items} actions={Object.values(VolumeGroupActions)} />;
  },

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const { volumes, capabilities, isLoading, error, refetch, isFetching } = useVolumesGroup(platformId);
    const items = useBackupStatusFilter(volumes?.volumes ?? EMPTY_VOLUMES);
    return { error, refetch, isFetching, items, isLoading, capabilities };
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
