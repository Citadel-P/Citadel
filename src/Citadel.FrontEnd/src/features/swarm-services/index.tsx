import { ActionBar } from '@/components/custom/action-bar';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { CitadelIcons } from '@/lib/icons';
import { SwarmServiceDropdownActions, SwarmServiceGroupActions } from './actions';
import { SwarmServicesTable } from './table';
import { useSwarmServicesGroup } from './hooks/useSwarmServicesGroup';

const EMPTY: never[] = [];

export const SwarmServiceComponents: RequiredComponents = {
  Icon: CitadelIcons.SwarmService,
  header: {
    title: 'Swarm Services',
    subtitle: 'Create and manage first-class Docker Swarm Services.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
  },
  Content: SwarmServicesTable,
  DropdownActions: SwarmServiceDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="SwarmService" items={items} actions={Object.values(SwarmServiceGroupActions)} />
  ),
  useData: (): ResourceDataHookResult<any> => {
    const { services, capabilities, isLoading } = useSwarmServicesGroup();
    return { items: services ?? EMPTY, capabilities, isLoading };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter((item) =>
          item.name.toLowerCase().includes(value) ||
          item.dockerName.toLowerCase().includes(value) ||
          item.id.toLowerCase().includes(value))
      : items;
  },
};
