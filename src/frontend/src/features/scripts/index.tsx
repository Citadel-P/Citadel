import { AuthorizedScript } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ScriptDropdownActions, ScriptGroupActions } from './actions';
import { ScriptsTable } from './table';
import { useScriptsGroup } from './hooks/useScriptsGroup';

const EMPTY_SCRIPTS: never[] = [];

export const ScriptComponents: RequiredComponents<AuthorizedScript> = {
  Icon: CitadelIcons.Script,
  Content: ({ items, actions, isLoading }) => <ScriptsTable items={items} actions={actions} isLoading={isLoading} />,
  header: {
    title: 'Scripts',
    subtitle: 'Run TypeScript scripts against Citadel resources.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Script',
  },
  DropdownActions: ScriptDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Script" items={items} actions={Object.values(ScriptGroupActions)} />,
  useData(): ResourceDataHookResult<AuthorizedScript> {
    const { scripts, isLoading, capabilities, error, refetch, isFetching } = useScriptsGroup();
    return { error, refetch, isFetching, items: scripts ?? EMPTY_SCRIPTS, isLoading, capabilities };
  },
  filterItems: filterScripts,
};

function filterScripts(items: AuthorizedScript[], search: string) {
  if (!search.trim()) return items;

  const s = search.toLowerCase();
  return items.filter(
    (script) =>
      script.name.toLowerCase().includes(s) ||
      (script.description ?? '').toLowerCase().includes(s) ||
      (script.scheduleCron ?? '').toLowerCase().includes(s),
  );
}
