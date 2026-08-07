import { RegularResourceComponents } from '@/pages/types';
import { Boxes } from 'lucide-react';
import { SwarmServiceListView, useServicesGroup } from './hooks/useServicesGroup';
import { ServicesTable } from './table';

export const ServiceComponents: RegularResourceComponents<SwarmServiceListView> = {
  Icon: Boxes,
  header: {
    title: 'Services',
    subtitle: 'Swarm services and current replica state.',
    showAdd: true,
    showSearch: true,
  },
  Content: ({ items, isLoading }) => <ServicesTable items={items} isLoading={isLoading} />,
  useData: (platformId) => {
    const { items, capabilities, isLoading } = useServicesGroup(platformId);
    return { items, isLoading, capabilities };
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
