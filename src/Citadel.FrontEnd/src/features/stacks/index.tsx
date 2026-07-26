import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { StackDropdownActions, StackGroupActions } from './actions';
import { StacksTable } from './table';
import { useStacksGroup } from './hooks/useStacksGroup';
import { CitadelIcons } from '@/lib/icons';
import { UpdatesAvailableFilter, useUpdatesAvailableFilter } from '@/components/custom/updates-available-filter';
import { hasStackUpdateAvailable } from './update-status';

const EMPTY_STACKS: never[] = [];

export const StackComponents: RequiredComponents = {
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
  Content: StackListContent,
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

function StackListContent({ items, actions, isLoading }: React.ComponentProps<typeof StacksTable>) {
  const { updatesAvailableOnly } = useUpdatesAvailableFilter();
  const visibleItems = updatesAvailableOnly ? items.filter(hasStackUpdateAvailable) : items;
  const emptyState = updatesAvailableOnly
    ? {
        title: 'No updates available',
        description: 'No available updates were detected for the current filters.',
      }
    : undefined;

  return <StacksTable items={visibleItems} actions={actions} isLoading={isLoading} emptyState={emptyState} />;
}
