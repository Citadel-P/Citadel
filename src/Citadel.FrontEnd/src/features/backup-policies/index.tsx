import { BackupPolicyView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { Separator } from '@/components/ui/separator';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { BackupPolicyDropdownActions, BackupPolicyGroupActions } from './actions';
import { useBackupPoliciesGroup } from './hooks/useBackupPoliciesGroup';
import { BackupRepositoriesSection } from './repositories';
import { BackupPoliciesTable } from './table';

export const BackupPolicyComponents: RequiredComponents<BackupPolicyView> = {
  Icon: CitadelIcons.BackupPolicy,
  Content: ({ items, actions, isLoading }) => (
    <div className="flex flex-col gap-6">
      <BackupPoliciesTable items={items} actions={actions} isLoading={isLoading} />
      <Separator className="bg-border/70" />
      <BackupRepositoriesSection />
    </div>
  ),
  header: {
    title: 'Backups',
    subtitle: 'Manage backup policies and encrypted repositories.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Policy',
  },
  DropdownActions: BackupPolicyDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="BackupPolicy" items={items} actions={Object.values(BackupPolicyGroupActions)} />
  ),
  useData(): ResourceDataHookResult<BackupPolicyView> {
    const { policies, isLoading, capabilities } = useBackupPoliciesGroup();
    return { items: policies, isLoading, capabilities };
  },
  filterItems: filterBackupPolicies,
};

function filterBackupPolicies(items: BackupPolicyView[], search: string) {
  if (!search.trim()) return items;

  const value = search.toLowerCase();
  return items.filter(
    (policy) =>
      policy.name.toLowerCase().includes(value) ||
      (policy.description ?? '').toLowerCase().includes(value) ||
      (policy.cron ?? '').toLowerCase().includes(value) ||
      sourceText(policy).toLowerCase().includes(value),
  );
}

function sourceText(policy: BackupPolicyView) {
  if (policy.source.$type === 'DockerVolume') return policy.source.volumeName;
  if (policy.source.$type === 'CitadelSystem') return 'citadel system';
  if (policy.source.$type === 'Deployment') return policy.source.deploymentId;
  return '';
}
