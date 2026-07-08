import { AutomationActionView, ResourceCapabilities } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { RequiredComponents, ResourceDataHookResult, TabbedResourceComponents } from '@/pages/types';
import { AutomationActionDropdownActions, AutomationActionGroupActions } from './actions';
import { AutomationActionsTable } from './table';

const AUTOMATION_CAPABILITIES: ResourceCapabilities = { canRead: true, canWrite: true, canExecute: true };

export const AutomationActionComponents: RequiredComponents<AutomationActionView> = {
  Icon: CitadelIcons.AutomationAction,
  Content: ({ items, actions, isLoading }) => (
    <AutomationActionsTable items={items} actions={actions} isLoading={isLoading} />
  ),
  header: {
    title: 'Actions',
    subtitle: 'Run TypeScript automations against Citadel resources.',
    showSearch: true,
    showAdd: true,
    addButtonTitle: 'Add Action',
  },
  DropdownActions: AutomationActionDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="AutomationAction" items={items} actions={Object.values(AutomationActionGroupActions)} />
  ),
  useData(): ResourceDataHookResult<AutomationActionView> {
    const { data, isLoading } = useRead('listAutomationActions');
    return { items: data?.data.actions ?? [], isLoading, capabilities: AUTOMATION_CAPABILITIES };
  },
  filterItems: filterAutomationActions,
};

export const AutomationComponents: TabbedResourceComponents = {
  Icon: CitadelIcons.Automation,
  header: {
    title: 'Automation',
    subtitle: 'Create and run operational actions.',
    showSearch: false,
    showAdd: false,
  },
  Tabs: [
    {
      label: 'Actions',
      slug: 'actions',
      Content: ({ items, actions, isLoading }) => (
        <AutomationActionsTable items={items} actions={actions} isLoading={isLoading} />
      ),
      Header: {
        showAdd: true,
        showSearch: true,
        addButtonTitle: 'Add Action',
        addButtonUrl: '/automation/actions/add',
      },
      DropdownActions: AutomationActionDropdownActions,
      GroupActions: ({ items }) => (
        <ActionBar type="AutomationAction" items={items} actions={Object.values(AutomationActionGroupActions)} />
      ),
      useData: () => {
        const { data, isLoading } = useRead('listAutomationActions');
        return { items: data?.data.actions ?? [], isLoading, capabilities: AUTOMATION_CAPABILITIES };
      },
    },
  ],
  filterItems: filterAutomationActions,
};

function filterAutomationActions(items: AutomationActionView[], search: string) {
  if (!search.trim()) return items;

  const s = search.toLowerCase();
  return items.filter(
    (action) =>
      action.name.toLowerCase().includes(s) ||
      (action.description ?? '').toLowerCase().includes(s) ||
      (action.scheduleCron ?? '').toLowerCase().includes(s),
  );
}
