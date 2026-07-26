import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { DeploymentDropdownActions, DeploymentGroupActions } from './actions';
import { DeploymentsTable } from './table';
import { useDeploymentsGroup } from './hooks/useDeploymentsGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter, useUpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { hasDeploymentUpdateAvailable } from './update-status';

const EMPTY_DEPLOYMENTS: never[] = [];

export const DeploymentComponents: RequiredComponents = {
  Icon: CitadelIcons.Deployment,
  header: {
    subtitle: 'Run and manage containers on your servers.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
    Extra: UpdatesAvailableFilter,
    activeFilterParams: ['updates'],
  },
  Content: DeploymentListContent,
  DropdownActions: DeploymentDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Deployment" items={items} actions={Object.values(DeploymentGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { deployments, capabilities, isLoading } = useDeploymentsGroup();
    return { items: deployments ?? EMPTY_DEPLOYMENTS, isLoading, capabilities };
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

function DeploymentListContent({ items, actions, isLoading }: React.ComponentProps<typeof DeploymentsTable>) {
  const { updatesAvailableOnly } = useUpdatesAvailableFilter();
  const visibleItems = updatesAvailableOnly ? items.filter(hasDeploymentUpdateAvailable) : items;
  const emptyState = updatesAvailableOnly
    ? {
        title: 'No updates available',
        description: 'No available updates were detected for the current filters.',
      }
    : undefined;

  return <DeploymentsTable items={visibleItems} actions={actions} isLoading={isLoading} emptyState={emptyState} />;
}
