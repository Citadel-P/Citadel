import { type PlatformView } from '@/api/generated/api.types';
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
