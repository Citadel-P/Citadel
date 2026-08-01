import { Box } from 'lucide-react';
import { ContainersTable } from './table';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useContainersGroup } from './hooks/useContainersGroup';
import { ContainerDropdownActions, ContainerGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import { useSearchParams } from 'react-router';
import { useMemo } from 'react';
import { isUnmanagedContainer } from '@/lib/utils';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';

const EMPTY_CONTAINERS: never[] = [];
const standaloneContainerActions = [ContainerGroupActions.adopt, ContainerGroupActions.importStack];
const groupedContainerActions = Object.values(ContainerGroupActions).filter(
  (action) => !standaloneContainerActions.includes(action),
);

const UnmanagedContainersFilter = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const checked = searchParams.get('unmanaged') === 'true';

  const handleCheckedChange = (next: boolean) => {
    setSearchParams(
      (current) => {
        const updated = new URLSearchParams(current);

        if (next) {
          updated.set('unmanaged', 'true');
        } else {
          updated.delete('unmanaged');
        }

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

export const ContainerComponents: RequiredComponents = {
  Icon: Box,
  Content: ({ items, isLoading, actions }) => {
    return <ContainersTable items={items} isLoading={isLoading} actions={actions} />;
  },

  useData: function (platformId: string): ResourceDataHookResult<any> {
    const [searchParams] = useSearchParams();
    const { containersInfo, capabilities, isLoading } = useContainersGroup(platformId);
    const unmanagedOnly = searchParams.get('unmanaged') === 'true';
    const containers = containersInfo?.containers ?? EMPTY_CONTAINERS;
    const items = useMemo(
      () => (unmanagedOnly ? containers.filter(isUnmanagedContainer) : containers),
      [containers, unmanagedOnly],
    );

    return {
      items,
      isLoading,
      capabilities,
    };
  },
  header: {
    showAdd: false,
    showSearch: true,
    Extra: UnmanagedContainersFilter,
  },
  DropdownActions: ContainerDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar
      type="Container"
      items={items}
      actions={groupedContainerActions}
      standaloneActions={standaloneContainerActions}
    />
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
