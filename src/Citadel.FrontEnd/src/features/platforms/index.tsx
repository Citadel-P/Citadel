import { Server } from 'lucide-react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { usePlatformsGroup } from './hooks/usePlatformsGroup';
import { Platforms } from './platforms';
import { PlatformDropdownActions } from './actions';

export const PlatformComponents: RequiredComponents = {
  Icon: Server,
  header: {
    subtitle: 'Connect platforms for real-time monitoring, alerts, and container workloads.',
    showSearch: true,
    showAdd: true,
  },
  Content: ({ items, actions, isLoading, isFiltered }) => {
    return <Platforms items={items} actions={actions} isLoading={isLoading} isFiltered={isFiltered} />;
  },
  DropdownActions: PlatformDropdownActions,
  useData: function (): ResourceDataHookResult<any> {
    const { platformsMessage, isLoading } = usePlatformsGroup();
    return { items: platformsMessage ?? [], isLoading };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter((v) => v.name?.toLowerCase().includes(s) || v.id?.toLowerCase().includes(s));
  },
};
