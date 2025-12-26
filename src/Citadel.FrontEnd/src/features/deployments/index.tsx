import { Cable } from 'lucide-react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { ActionBar } from '@/components/custom/action-bar';
import { DeploymentDropdownActions, DeploymentGroupActions } from './actions';
import { DeploymentsTable } from './table';

export const DeploymentComponents: RequiredComponents = {
  Icon: <Cable className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return <DeploymentsTable items={items} actions={actions} isLoading={isLoading} />;
  },
  DropdownActions: DeploymentDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Deployment" items={items} actions={Object.values(DeploymentGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { data, isLoading } = useRead(`listDeployments`);
    return { items: data?.data?.deployments ?? [], isLoading };
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
