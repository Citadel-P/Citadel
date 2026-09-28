import { DeploymentStatus, type DeploymentView } from '@/api/generated/api.types';
import { Rocket, CircleCheck, TriangleAlert, CircleStop, ArrowUpCircle } from 'lucide-react';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { DeploymentDropdownActions, DeploymentGroupActions } from './actions';
import { DeploymentsTable } from './table';
import { useDeploymentsGroup } from './hooks/useDeploymentsGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
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
    Extra: UpdatesAvailableFilter,
    activeFilterParams: ['updates'],
  },
  overview: {
    label: 'Deployment overview',
    filters: [
      { id: 'all', label: 'All deployments', description: 'All matching deployments', icon: Rocket },
      {
        id: 'healthy',
        label: 'Healthy',
        description: 'Workloads running normally',
        icon: CircleCheck,
        tone: 'success',
        matches: (item) => item.status === DeploymentStatus.Healthy,
      },
      {
        id: 'attention',
        label: 'Needs attention',
        description: 'Degraded or failed deployments',
        icon: TriangleAlert,
        tone: 'warning',
        matches: (item) => [DeploymentStatus.Degraded, DeploymentStatus.Failed].includes(item.status),
      },
      {
        id: 'stopped',
        label: 'Stopped',
        description: 'Stopped deployments',
        icon: CircleStop,
        matches: (item) => item.status === DeploymentStatus.Stopped,
      },
      {
        id: 'updates',
        label: 'Updates available',
        description: 'Detected image updates',
        icon: ArrowUpCircle,
        matches: hasDeploymentUpdateAvailable,
      },
    ],
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
              description: 'Select another overview card or adjust the search and filters.',
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
    return { error, refetch, isFetching, items: deployments ?? EMPTY_DEPLOYMENTS, isLoading, capabilities };
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
