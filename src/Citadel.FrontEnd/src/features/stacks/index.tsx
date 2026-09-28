import { RegularResourceComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { StackDropdownActions, StackGroupActions } from './actions';
import { StacksTable } from './table';
import { useStacksGroup } from './hooks/useStacksGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { hasStackUpdateAvailable } from './update-status';
import { StackReleaseStatus, type StackView } from '@/api/generated/api.types';
import { Layers, CircleCheck, TriangleAlert, ArrowUpCircle, CircleStop } from 'lucide-react';

const EMPTY_STACKS: never[] = [];

export const StackComponents: RegularResourceComponents<StackView> = {
  Icon: CitadelIcons.Stack,
  header: {
    subtitle: 'Run and manage stacks on your servers.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    showPlatformFilter: true,
    Extra: UpdatesAvailableFilter,
    activeFilterParams: ['updates'],
  },
  overview: {
    label: 'Stack overview',
    filters: [
      { id: 'all', label: 'All stacks', description: 'All matching stacks', icon: Layers },
      {
        id: 'healthy',
        label: 'Healthy',
        description: 'Workloads running normally',
        icon: CircleCheck,
        tone: 'success',
        matches: (item) => item.status === StackReleaseStatus.Healthy,
      },
      {
        id: 'attention',
        label: 'Needs attention',
        description: 'Degraded, failed or timed out',
        icon: TriangleAlert,
        tone: 'warning',
        matches: (item) =>
          [StackReleaseStatus.Degraded, StackReleaseStatus.Failed, StackReleaseStatus.TimedOut].includes(item.status),
      },
      {
        id: 'stopped',
        label: 'Stopped',
        description: 'Stopped stacks',
        icon: CircleStop,
        matches: (item) => item.status === StackReleaseStatus.Stopped,
      },
      {
        id: 'updates',
        label: 'Updates available',
        description: 'Detected image or Git changes',
        icon: ArrowUpCircle,
        matches: hasStackUpdateAvailable,
      },
    ],
  },
  Content: ({ items, actions, isLoading, isFiltered }) => (
    <StacksTable
      items={items}
      actions={actions}
      isLoading={isLoading}
      emptyState={
        isFiltered
          ? {
              title: 'No stacks match the current filters.',
              description: 'Select another overview card or adjust the search and filters.',
            }
          : undefined
      }
    />
  ),
  DropdownActions: StackDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Stack" items={items} actions={Object.values(StackGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { stacks, capabilities, isLoading } = useStacksGroup();
    return { items: stacks ?? EMPTY_STACKS, isLoading, capabilities };
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
