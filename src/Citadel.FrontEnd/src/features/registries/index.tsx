import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { ActionBar } from '@/components/custom/action-bar';
import { RegistryDropdownActions, RegistryGroupActions } from './actions';
import { RegistriesTable } from './table';
import { CitadelIcons } from '@/lib/icons';

export const RegistryComponents: RequiredComponents = {
  Icon: CitadelIcons.Registry,
  Content: ({ items, actions, isLoading }) => {
    return <RegistriesTable items={items} actions={actions} isLoading={isLoading} />;
  },
  header: {
    subtitle: 'Connect and configure container image registries.',
    showSearch: true,
    showAdd: true,
  },
  DropdownActions: RegistryDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Registry" items={items} actions={Object.values(RegistryGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { data, isLoading } = useRead(`listRegistries`, { query: { includeDisabled: true } });
    return { items: data?.data?.registries ?? [], isLoading };
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
