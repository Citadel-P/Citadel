import { RegularResourceComponents } from '@/pages/types';
import { FileCode2 } from 'lucide-react';
import { useConfigsGroup } from './hooks/useConfigsGroup';
import { ConfigsTable } from './table';

export const ConfigComponents: RegularResourceComponents = {
  Icon: FileCode2,
  header: {
    title: 'Configs',
    subtitle: 'Swarm config metadata and service references.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, isLoading }) => <ConfigsTable items={items} isLoading={isLoading} />,
  useData: (platformId) => {
    const { items, isLoading } = useConfigsGroup(platformId);
    return { items, isLoading, capabilities: undefined };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter((item) => item.name?.toLowerCase().includes(value) || item.id?.toLowerCase().includes(value))
      : items;
  },
};
