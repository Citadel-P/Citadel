import { RegularResourceComponents } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { FileCode2 } from 'lucide-react';
import { useConfigsGroup } from './hooks/useConfigsGroup';
import { ConfigsTable } from './table';
import { ConfigDropdownActions, ConfigGroupActions } from './actions';

export const ConfigComponents: RegularResourceComponents = {
  Icon: FileCode2,
  header: {
    title: 'Configs',
    subtitle: 'Swarm config metadata and service references.',
    showAdd: true,
    showSearch: true,
  },
  Content: ({ items, actions, isLoading }) => <ConfigsTable items={items} actions={actions} isLoading={isLoading} />,
  DropdownActions: ConfigDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Config" items={items} actions={Object.values(ConfigGroupActions)} />,
  useData: (platformId) => {
    const { items, capabilities, isLoading } = useConfigsGroup(platformId);
    return { items, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter((item) => item.name?.toLowerCase().includes(value) || item.id?.toLowerCase().includes(value))
      : items;
  },
};
