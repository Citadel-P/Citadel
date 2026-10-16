import { withContainerGroupHealth } from './group-health';
import { ContainerHealthFilter, useContainerHealthFilter, useHealthClock } from './health';
import { BackupStatusFilter, useBackupStatusFilter } from '@/features/backup-policies/table-coverage';
import { filterContainers } from './container-search';
import { Box } from 'lucide-react';
import { ContainersTable } from './table';
import { ContainerColumnsButton, containerColumnsAtom } from './table-columns';
import { useAtomValue } from 'jotai';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { useContainersGroup } from './hooks/useContainersGroup';
import { ContainerDropdownActions, ContainerGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import { useSearchParams } from 'react-router';
import { useMemo } from 'react';
import { isUnmanagedContainer } from '@/lib/utils';
import { countSelectedContainers, filterVisibleContainers } from './selection';

const EMPTY_CONTAINERS: never[] = [];
const standaloneContainerActions = [ContainerGroupActions.adopt, ContainerGroupActions.importStack];
const groupedContainerActions = Object.values(ContainerGroupActions).filter(
  (action) => !standaloneContainerActions.includes(action),
);

export const ContainerComponents: RegularResourceComponents = {
  Icon: Box,

  Content: ContainersTable,

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const [searchParams] = useSearchParams();
    const { containersInfo, capabilities, isLoading, error, refetch, isFetching } = useContainersGroup(platformId);
    const unmanagedOnly = searchParams.get('unmanaged') === 'true';
    const containers = containersInfo?.containers ?? EMPTY_CONTAINERS;
    const healthNow = useHealthClock();
    const items = useMemo(() => {
      const visibleContainers = withContainerGroupHealth(filterVisibleContainers(containers), healthNow);
      return unmanagedOnly ? visibleContainers.filter(isUnmanagedContainer) : visibleContainers;
    }, [containers, unmanagedOnly, healthNow]);

    const visibleColumns = useAtomValue(containerColumnsAtom);
    const filteredItems = useContainerHealthFilter(useBackupStatusFilter(items), visibleColumns.health !== false);
    return { error, refetch, isFetching, items: filteredItems, isLoading, capabilities };
  },
  header: {
    subtitle: 'Monitor container state, resource usage, and workload operations.',
    activeFilterParams: ['unmanaged', 'backupStatus', 'health'],
    showAdd: false,
    showSearch: true,
    Extra: () => {
      const visibleColumns = useAtomValue(containerColumnsAtom);
      return (
        <>
          <UnmanagedContainersFilter />
          <BackupStatusFilter />
          {visibleColumns.health !== false && <ContainerHealthFilter />}
          <ContainerColumnsButton />
        </>
      );
    },
  },
  DropdownActions: ContainerDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar
      type="Container"
      items={items}
      actions={groupedContainerActions}
      standaloneActions={standaloneContainerActions}
      getSelectedCount={countSelectedContainers}
    />
  ),

  filterItems: filterContainers,
};

const UnmanagedContainersFilter = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const checked = searchParams.get('unmanaged') === 'true';
  const handleCheckedChange = (next: boolean) => {
    setSearchParams(
      (current) => {
        const updated = new URLSearchParams(current);
        if (next) updated.set('unmanaged', 'true');
        else updated.delete('unmanaged');
        return updated;
      },
      { replace: true },
    );
  };
  return (
    <div className="inline-flex h-9 items-center gap-2 rounded-sm border bg-background px-2.5 text-xs text-muted-foreground">
      <Switch
        id="unmanaged-containers-only"
        checked={checked}
        onCheckedChange={handleCheckedChange}
        aria-label="Show unmanaged containers only"
      />
      <Label
        htmlFor="unmanaged-containers-only"
        className={`whitespace-nowrap text-xs font-normal ${checked ? 'text-foreground' : ''}`}>
        Unmanaged only
      </Label>
    </div>
  );
};
