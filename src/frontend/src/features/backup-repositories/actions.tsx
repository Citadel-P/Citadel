import { useRemoveResourceAction } from '@/components/custom/useSettledResourceAction';
import { BackupRepositoryView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useMutate } from '@/lib/hooks';
import { CheckCircle2, DatabaseZap, Eye, RefreshCw, Scissors, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useRepositoryOperations, type RepositoryOperation } from './operations';

const singleSelection = (resources: BackupRepositoryView | BackupRepositoryView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const useRepositoryOperation = (
  resources: BackupRepositoryView | BackupRepositoryView[],
  operation: RepositoryOperation,
) => {
  const { selected, multiSelect } = singleSelection(resources);
  const { canRun, run, pending } = useRepositoryOperations(selected);
  return {
    canExecute: canRun(operation, multiSelect),
    isPending: pending === operation,
    run: () => run(operation, multiSelect),
  };
};

const { dropdown, group, info } = createActionsBuilder<BackupRepositoryView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Eye,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          navigate(`/backup-repositories/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'validate',
    type: 'command',
    icon: CheckCircle2,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'validate'),
  })
  .addAction({
    key: 'initialize',
    type: 'command',
    icon: DatabaseZap,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'initialize'),
  })
  .addAction({
    key: 'check',
    type: 'command',
    icon: RefreshCw,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'check'),
  })
  .addAction({
    key: 'prune',
    type: 'command',
    icon: Scissors,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'prune'),
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash2,
    confirm: true,
    destructive: true,
    separatorBefore: true,
    resourceType: 'BackupRepository',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const archive = useMutate('archiveBackupRepository');
      return useRemoveResourceAction(selected, {
        type: 'BackupRepository',
        listPath: '/backup-repositories',
        mutate: (item) => archive.mutateAsync({ id: item.id }),
        message: (count, total) => `${count} of ${total} ${total === 1 ? 'repository' : 'repositories'} archived`,
      });
    },
  })
  .build();

export const BackupRepositoryDropdownActions = dropdown;
export const BackupRepositoryGroupActions = group;
export const BackupRepositoryInfoActions = info;
