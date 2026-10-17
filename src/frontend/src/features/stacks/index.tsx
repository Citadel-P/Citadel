import { WorkloadHealthFilter, WorkloadHealthColumns, useWorkloadHealthFilter } from '@/features/workload-health/table';
import { useMemo } from 'react';
import { BackupStatusFilter, useBackupStatusFilter } from '@/features/backup-policies/table-coverage';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { StackDropdownActions, StackGroupActions } from './actions';
import { StacksTable } from './table';
import { useStacksGroup } from './hooks/useStacksGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter, useUpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { hasStackUpdateAvailable } from './update-status';
import { type StackView } from '@/api/generated/api.types';

const EMPTY_STACKS: never[] = [];

export const StackComponents: RegularResourceComponents<StackView> = {
  Icon: CitadelIcons.Stack,
  header: {
    subtitle: 'Run and manage stacks on your servers.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
    Extra: () => (
      <>
        <UpdatesAvailableFilter />
        <BackupStatusFilter />
        <WorkloadHealthFilter kind="Stack" />
        <WorkloadHealthColumns kind="Stack" />
      </>
    ),
    activeFilterParams: ['updates', 'backupStatus', 'health'],
  },

  Content: ({ items, actions, isLoading, isFiltered }) => (
    <StacksTable
      items={items}
      actions={actions}
      isLoading={isLoading}
      emptyState={
        isFiltered
          ? {
              title: 'No stacks match the current filters.',
              description: 'Adjust the search and filters.',
            }
          : undefined
      }
    />
  ),
  DropdownActions: StackDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Stack" items={items} actions={Object.values(StackGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { stacks, capabilities, isLoading, error, refetch, isFetching } = useStacksGroup();
    const { updatesAvailableOnly } = useUpdatesAvailableFilter();
    const backupItems = useBackupStatusFilter(stacks ?? EMPTY_STACKS);
    const coveredItems = useWorkloadHealthFilter(backupItems, 'Stack');
    const items = useMemo(
      () => (updatesAvailableOnly ? coveredItems.filter(hasStackUpdateAvailable) : coveredItems),
      [coveredItems, updatesAvailableOnly],
    );
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
