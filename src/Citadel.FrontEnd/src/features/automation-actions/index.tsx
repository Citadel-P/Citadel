import { AuthorizedAction } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { AutomationActionDropdownActions, AutomationActionGroupActions } from './actions';
import { AutomationActionsTable } from './table';
import { useAutomationActionsGroup } from './hooks/useAutomationActionsGroup';

const EMPTY_AUTOMATION_ACTIONS: never[] = [];

export const AutomationActionComponents: RequiredComponents<AuthorizedAction> = {
  Icon: CitadelIcons.AutomationAction,
  Content: ({ items, actions, isLoading }) => (
    <AutomationActionsTable items={items} actions={actions} isLoading={isLoading} />
  ),
  header: {
    title: 'Automation',
    subtitle: 'Run TypeScript automations against Citadel resources.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Action',
  },
  DropdownActions: AutomationActionDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="AutomationAction" items={items} actions={Object.values(AutomationActionGroupActions)} />
  ),
  useData(): ResourceDataHookResult<AuthorizedAction> {
    const { actions, isLoading, capabilities, error, refetch, isFetching } = useAutomationActionsGroup();
    return { error, refetch, isFetching, items: actions ?? EMPTY_AUTOMATION_ACTIONS, isLoading, capabilities };
  },
  filterItems: filterAutomationActions,
};

function filterAutomationActions(items: AuthorizedAction[], search: string) {
  if (!search.trim()) return items;

  const s = search.toLowerCase();
  return items.filter(
    (action) =>
      action.name.toLowerCase().includes(s) ||
      (action.description ?? '').toLowerCase().includes(s) ||
      (action.scheduleCron ?? '').toLowerCase().includes(s),
  );
}
