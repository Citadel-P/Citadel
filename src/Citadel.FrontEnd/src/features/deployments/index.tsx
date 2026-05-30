import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { DeploymentDropdownActions, DeploymentGroupActions } from './actions';
import { DeploymentsTable } from './table';
import { useDeploymentsGroup } from './hooks/useDeploymentsGroup';
import { CitadelIcons } from '@/lib/icons';

export const DeploymentComponents: RequiredComponents = {
  Icon: CitadelIcons.Deployment,
  header: {
    subtitle: 'Run and manage containers on your servers.',
    showSearch: true,
    showAdd: true,
  },
  Content: ({ items, actions, isLoading }) => {
    return <DeploymentsTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: DeploymentDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Deployment" items={items} actions={Object.values(DeploymentGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { deployments, capabilities, isLoading } = useDeploymentsGroup();
    return { items: deployments ?? [], isLoading, capabilities };
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
