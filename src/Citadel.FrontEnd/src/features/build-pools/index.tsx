import { BuildAgentPoolView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { useResourceTagFilter } from '@/features/tags/components';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { BuildPoolDropdownActions, BuildPoolGroupActions } from './actions';
import { BuildPoolsTable } from './table';

const EMPTY_BUILD_POOLS: never[] = [];

export const BuildPoolComponents: RequiredComponents<BuildAgentPoolView> = {
  Icon: CitadelIcons.BuildAgentPool,
  Content: ({ items, actions, isLoading }) => <BuildPoolsTable items={items} actions={actions} isLoading={isLoading} />,
  header: {
    title: 'Build Pools',
    subtitle: 'Configure external builders for Citadel build projects.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Pool',
  },
  DropdownActions: BuildPoolDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="BuildAgentPool" items={items} actions={Object.values(BuildPoolGroupActions)} />
  ),
  useData(): ResourceDataHookResult<BuildAgentPoolView> {
    const { selectedTagNames } = useResourceTagFilter();
    const { data, isLoading } = useRead('listBuildAgentPools', {
      query: selectedTagNames.length > 0 ? { tags: selectedTagNames } : undefined,
    });
    return { items: data?.data?.pools ?? EMPTY_BUILD_POOLS, isLoading, capabilities: data?.data?.capabilities };
  },
  filterItems: filterBuildPools,
};

function filterBuildPools(items: BuildAgentPoolView[], search: string) {
  if (!search.trim()) return items;

  const value = search.toLowerCase();
  return items.filter(
    (pool) =>
      pool.name.toLowerCase().includes(value) ||
      (pool.description ?? '').toLowerCase().includes(value) ||
      pool.provider.toLowerCase().includes(value),
  );
}
