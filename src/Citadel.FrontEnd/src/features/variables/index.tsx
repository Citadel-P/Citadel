import { ConfigurationEntryView, ResourceCapabilities } from '@/api/generated/api.types';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { VariablesAddButton, VariablesGroupActions, VariablesTable } from './table';

const EMPTY_CAPABILITIES: ResourceCapabilities = { canRead: false, canWrite: false, canExecute: false };

export const VariableComponents: RequiredComponents<ConfigurationEntryView> = {
  Icon: CitadelIcons.Variable,
  Content: ({ items, isLoading, isFiltered }) => (
    <VariablesTable items={items} isLoading={isLoading} isFiltered={isFiltered} />
  ),
  GroupActions: VariablesGroupActions,
  header: {
    title: 'Variables',
    subtitle: 'Manage global variables and stored secret keys inherited by stacks and deployments.',
    showSearch: true,
    showAdd: false,
    Extra: VariablesAddButton,
  },
  useData(): ResourceDataHookResult<ConfigurationEntryView> {
    const { data, isLoading } = useRead('getGlobalConfigurationEntries');
    return { items: data?.data.entries ?? [], isLoading, capabilities: data?.data.capabilities ?? EMPTY_CAPABILITIES };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (entry) =>
        entry.name?.toLowerCase().includes(s) ||
        entry.kind?.toLowerCase().includes(s) ||
        entry.value?.toLowerCase().includes(s),
    );
  },
};
