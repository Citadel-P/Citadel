import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { StackDropdownActions, StackGroupActions } from './actions';
import { StacksTable } from './table';
import { useStacksGroup } from './hooks/useStacksGroup';
import { CitadelIcons } from '@/lib/icons';

export const StackComponents: RequiredComponents = {
  Icon: CitadelIcons.Stack,
  header: {
    subtitle: 'Run and manage stacks on your servers.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
  },
  Content: ({ items, actions, isLoading }) => {
    return <StacksTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: StackDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Stack" items={items} actions={Object.values(StackGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { stacks, capabilities, isLoading } = useStacksGroup();
    return { items: stacks ?? [], isLoading, capabilities };
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
