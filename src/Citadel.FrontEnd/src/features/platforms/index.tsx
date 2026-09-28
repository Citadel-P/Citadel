import { PlatformStatus, type PlatformView } from '@/api/generated/api.types';
import { Server, CircleCheck, Unplug } from 'lucide-react';
import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { usePlatformsGroup } from './hooks/usePlatformsGroup';
import { Platforms } from './platforms';
import { PlatformDropdownActions } from './actions';
import { CitadelIcons } from '@/lib/icons';

const EMPTY_PLATFORMS: never[] = [];

export const PlatformComponents: RegularResourceComponents<PlatformView> = {
  Icon: CitadelIcons.Platform,
  header: {
    subtitle: 'Connect platforms for real-time monitoring, alerts, and container workloads.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonUrl: '/platforms/add',
  },
  overview: {
    label: 'Platform overview',
    filters: [
      { id: 'all', label: 'All platforms', description: 'All matching platforms', icon: Server },
      {
        id: 'online',
        label: 'Online',
        description: 'Connected and available',
        icon: CircleCheck,
        tone: 'success',
        matches: (item) => item.status === PlatformStatus.Online,
      },
      {
        id: 'offline',
        label: 'Offline',
        description: 'Connection unavailable',
        icon: Unplug,
        tone: 'warning',
        matches: (item) => item.status === PlatformStatus.Offline,
      },
    ],
  },
  Content: ({ items, actions, isLoading, isFiltered }) => {
    return <Platforms items={items} actions={actions} isLoading={isLoading} isFiltered={isFiltered} />;
  },
  DropdownActions: PlatformDropdownActions,
  useData: function (): ResourceDataHookResult<any> {
    const { platformsMessage, capabilities, isLoading, error, refetch, isFetching } = usePlatformsGroup();
    return { error, refetch, isFetching, items: platformsMessage ?? EMPTY_PLATFORMS, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter((v) => v.name?.toLowerCase().includes(s) || v.id?.toLowerCase().includes(s));
  },
};
