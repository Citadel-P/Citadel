import { RegularResourceComponents } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { Boxes } from 'lucide-react';
import { SwarmServiceListView, useServicesGroup } from './hooks/useServicesGroup';
import { ServicesTable } from './table';
import { ServiceDropdownActions, ServiceGroupActions } from './actions';

const { adopt, importStack, ...groupedServiceActions } = ServiceGroupActions;

export const ServiceComponents: RegularResourceComponents<SwarmServiceListView> = {
  Icon: Boxes,
  header: {
    title: 'Services',
    subtitle: 'Swarm services and current replica state.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, isLoading, actions }) => <ServicesTable items={items} isLoading={isLoading} actions={actions} />,
  DropdownActions: ServiceDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar
      type="Service"
      items={items}
      actions={Object.values(groupedServiceActions)}
      standaloneActions={[adopt, importStack]}
    />
  ),
  useData: (platformId) => {
    const { items, capabilities, isLoading, error, refetch, isFetching } = useServicesGroup(platformId);
    return { error, refetch, isFetching, items, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter(
          (item) =>
            item.name?.toLowerCase().includes(value) ||
            item.image?.toLowerCase().includes(value) ||
            item.id?.toLowerCase().includes(value),
        )
      : items;
  },
};
