import { SwarmServiceHealth, type ManagedSwarmServiceView } from '@/api/generated/api.types';
import { Boxes, CircleCheck, TriangleAlert, RefreshCw, CircleStop } from 'lucide-react';
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
  },
  overview: {
    label: 'Swarm service overview',
    filters: [
      { id: 'all', label: 'All services', description: 'All matching managed services', icon: Boxes },
      {
        id: 'healthy',
        label: 'Healthy',
        description: 'Services running normally',
        icon: CircleCheck,
        tone: 'success',
        matches: (item) => item.health === SwarmServiceHealth.Healthy,
      },
      {
        id: 'attention',
        label: 'Needs attention',
        description: 'Degraded or failed services',
        icon: TriangleAlert,
        tone: 'warning',
        matches: (item) => [SwarmServiceHealth.Degraded, SwarmServiceHealth.Failed].includes(item.health),
      },
      {
        id: 'updating',
        label: 'Updating',
        description: 'Deploying, reconciling or rolling back',
        icon: RefreshCw,
        matches: (item) =>
          item.health === SwarmServiceHealth.Progressing ||
          ['updating', 'rollback_started'].includes(item.updateState ?? ''),
      },
      {
        id: 'zero',
        label: 'Scaled to zero',
        description: 'Replicated services with zero desired replicas',
        icon: CircleStop,
        matches: (item) =>
          item.spec?.schedulingMode === 'Replicated' && item.spec.replicas != null && Number(item.spec.replicas) === 0,
      },
    ],
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
    const { services, capabilities, isLoading } = useSwarmServicesGroup();
    return { items: services ?? EMPTY, capabilities, isLoading };
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
