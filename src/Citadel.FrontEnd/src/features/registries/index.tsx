import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { ActionBar } from '@/components/custom/action-bar';
import { RegistryDropdownActions, RegistryGroupActions } from './actions';
import { RegistriesTable } from './table';
import { CitadelIcons } from '@/lib/icons';
import { useResourceTagFilter } from '@/features/tags/components';

const EMPTY_REGISTRIES: never[] = [];

export const RegistryComponents: RequiredComponents = {
  Icon: CitadelIcons.Registry,
  Content: ({ items, actions, isLoading }) => {
    return <RegistriesTable items={items} actions={actions} isLoading={isLoading} />;
  },
  header: {
    subtitle: 'Connect and configure container image registries.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
  },
  DropdownActions: RegistryDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Registry" items={items} actions={Object.values(RegistryGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { selectedTagNames } = useResourceTagFilter();
    const { data, isLoading, error, refetch, isFetching } = useRead(`listRegistries`, {
      query: { includeDisabled: true, ...(selectedTagNames.length > 0 ? { tags: selectedTagNames } : {}) },
    });
    return {
      error,
      refetch,
      isFetching,
      items: data?.data?.registries ?? EMPTY_REGISTRIES,
      isLoading,
      capabilities: data?.data?.capabilities,
    };
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
