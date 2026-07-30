import { TagView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { TagDropdownActions, TagGroupActions } from './actions';
import { TagCreateDialog } from './dialogs';
import { TagsTable } from './table';

const EMPTY_TAGS: never[] = [];

export const TagComponents: RequiredComponents<TagView> = {
  Icon: CitadelIcons.Tag,
  Content: ({ items, actions, isLoading }) => <TagsTable items={items} actions={actions} isLoading={isLoading} />,
  header: {
    subtitle: 'Create and maintain global tags used to organize resources.',
    showSearch: true,
    showAdd: true,
    AddDialog: TagCreateDialog,
  },
  DropdownActions: TagDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Tag" items={items} actions={Object.values(TagGroupActions)} />;
  },
  useData(): ResourceDataHookResult<TagView> {
    const { data, isLoading } = useRead('listTags');
    return {
      items: data?.data.tags ?? EMPTY_TAGS,
      isLoading,
      capabilities: data?.data.capabilities,
    };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (tag) =>
        tag.name.toLowerCase().includes(s) ||
        tag.normalizedName.toLowerCase().includes(s) ||
        tag.id.toLowerCase().includes(s),
    );
  },
};
