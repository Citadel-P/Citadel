import { WorkloadHealthFilter, WorkloadHealthColumns, useWorkloadHealthFilter } from '@/features/workload-health/table';
import { BackupStatusFilter, useBackupStatusFilter } from '@/features/backup-policies/table-coverage';
import { type ManagedSwarmServiceView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { CitadelIcons } from '@/lib/icons';
import { SwarmServiceDropdownActions, SwarmServiceGroupActions } from './actions';
import { SwarmServicesTable } from './table';
import { useSwarmServicesGroup } from './hooks/useSwarmServicesGroup';

const EMPTY: never[] = [];
const { duplicate, checkUpdates, ...groupedServiceActions } = SwarmServiceGroupActions;

export const SwarmServiceComponents: RegularResourceComponents<ManagedSwarmServiceView> = {
  Icon: CitadelIcons.SwarmService,
  header: {
    title: 'Swarm Services',
    subtitle: 'Create and manage first-class Docker Swarm Services.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
    Extra: () => (
      <>
        <BackupStatusFilter />
        <WorkloadHealthFilter kind="SwarmService" />
        <WorkloadHealthColumns kind="SwarmService" />
      </>
    ),
    activeFilterParams: ['backupStatus', 'health'],
  },

  Content: SwarmServicesTable,
  DropdownActions: SwarmServiceDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar
      type="SwarmService"
      items={items}
      actions={Object.values(groupedServiceActions)}
      standaloneActions={[duplicate, checkUpdates]}
    />
  ),
  useData: (): ResourceDataHookResult<any> => {
    const { services, capabilities, isLoading, error, refetch, isFetching } = useSwarmServicesGroup();
    const coveredItems = useBackupStatusFilter(services ?? EMPTY);
    const items = useWorkloadHealthFilter(coveredItems, 'SwarmService');
    return { error, refetch, isFetching, items, capabilities, isLoading };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter(
          (item) =>
            item.name.toLowerCase().includes(value) ||
            item.dockerName.toLowerCase().includes(value) ||
            item.id.toLowerCase().includes(value),
        )
      : items;
  },
};
