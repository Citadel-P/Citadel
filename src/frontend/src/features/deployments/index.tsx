import { useMemo } from 'react';
import { BackupStatusFilter, useBackupStatusFilter } from '@/features/backup-policies/table-coverage';
import { type DeploymentView } from '@/api/generated/api.types';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { DeploymentDropdownActions, DeploymentGroupActions } from './actions';
import { DeploymentsTable } from './table';
import { useDeploymentsGroup } from './hooks/useDeploymentsGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter, useUpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { hasDeploymentUpdateAvailable } from './update-status';

const EMPTY_DEPLOYMENTS: never[] = [];

export const DeploymentComponents: RegularResourceComponents<DeploymentView> = {
  Icon: CitadelIcons.Deployment,
  header: {
    subtitle: 'Run and manage containers on your servers.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
    Extra: () => (
      <>
        <UpdatesAvailableFilter />
        <BackupStatusFilter />
      </>
    ),
    activeFilterParams: ['updates', 'backupStatus'],
  },

  Content: ({ items, actions, isLoading, isFiltered }) => (
    <DeploymentsTable
      items={items}
      actions={actions}
      isLoading={isLoading}
      emptyState={
        isFiltered
          ? {
              title: 'No deployments match the current filters.',
              description: 'Adjust the search and filters.',
            }
          : undefined
      }
    />
  ),
  DropdownActions: DeploymentDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Deployment" items={items} actions={Object.values(DeploymentGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { deployments, capabilities, isLoading, error, refetch, isFetching } = useDeploymentsGroup();
    const { updatesAvailableOnly } = useUpdatesAvailableFilter();
    const coveredItems = useBackupStatusFilter(deployments ?? EMPTY_DEPLOYMENTS);
    const items = useMemo(
      () => (updatesAvailableOnly ? coveredItems.filter(hasDeploymentUpdateAvailable) : coveredItems),
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
