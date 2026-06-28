import { GlobalConfigurationEntriesTab } from '@/components/custom/configuration-entries-tab';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';

export const VariableComponents: RequiredComponents = {
  Icon: CitadelIcons.Variable,
  Content: () => <GlobalConfigurationEntriesTab />,
  header: {
    title: 'Variables',
    subtitle: 'Manage global variables and secret bindings inherited by stacks and deployments.',
    showSearch: false,
    showAdd: false,
  },
  useData(): ResourceDataHookResult<never> {
    return { items: [], isLoading: false, capabilities: undefined };
  },
};
