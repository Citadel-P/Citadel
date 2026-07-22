import { BuildAgentPoolView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { BuildPoolDropdownActions, BuildPoolGroupActions } from './actions';
import { BuildPoolsTable } from './table';
import { useBuildPoolsGroup } from './useBuildPoolsGroup';

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
    const { pools, isLoading, capabilities } = useBuildPoolsGroup();
    return { items: pools, isLoading, capabilities };
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
