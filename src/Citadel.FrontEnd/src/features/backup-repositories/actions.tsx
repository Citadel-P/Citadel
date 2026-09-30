import { BackupRepositoryView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, DatabaseZap, Eye, RefreshCw, Scissors, Trash2 } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { getRepositoryOperationContext } from './form/form';

const singleSelection = (resources: BackupRepositoryView | BackupRepositoryView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const invalidateBackupRepositoryQueries = async (queryClient: ReturnType<typeof useQueryClient>, id?: string) => {
  await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getBackupRepository'] });
  }
};

const useRepositoryOperation = (
  resources: BackupRepositoryView | BackupRepositoryView[],
  mutationKey:
    | 'validateBackupRepository'
    | 'initializeBackupRepository'
    | 'checkBackupRepository'
    | 'pruneBackupRepository',
  successMessage: string,
) => {
  const { selected, multiSelect } = singleSelection(resources);
  const queryClient = useQueryClient();
  const mutation = useMutate(mutationKey);

  return {
    canExecute: !!selected && !multiSelect,
    isPending: mutation.isPending,
    run: async () => {
      if (!selected || multiSelect) return;

      try {
        await mutation.mutateAsync({ id: selected.id, data: getRepositoryOperationContext(selected) } as any);
        await invalidateBackupRepositoryQueries(queryClient, selected.id);
        toast.success(successMessage);
      } catch {
        /** Nope */
      }
    },
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
    useHandler: ({ resources }) =>
      useRepositoryOperation(resources, 'validateBackupRepository', 'Repository validated'),
  })
  .addAction({
    key: 'initialize',
    type: 'command',
    icon: DatabaseZap,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) =>
      useRepositoryOperation(resources, 'initializeBackupRepository', 'Repository initialized'),
  })
  .addAction({
    key: 'check',
    type: 'command',
    icon: RefreshCw,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'checkBackupRepository', 'Repository checked'),
  })
  .addAction({
    key: 'prune',
    type: 'command',
    icon: Scissors,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => useRepositoryOperation(resources, 'pruneBackupRepository', 'Repository pruned'),
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
      const queryClient = useQueryClient();
      const archive = useMutate('archiveBackupRepository');
      const [, setSelectedResources] = useSelectedResources<BackupRepositoryView>('BackupRepository');
      const location = useLocation();
      const navigate = useNavigate();
      const isCurrentResource = selected.some(
        (repository) => location.pathname.replace(/\/$/, '') === `/backup-repositories/edit/${repository.id}`,
      );

      return {
        canExecute: selected.length > 0,
        isPending: archive.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((repository) => archive.mutateAsync({ id: repository.id } as any)));
            if (isCurrentResource) navigate('/backup-repositories', { replace: true });
            await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'repository' : 'repositories'} archived`);
          } catch {
            // toast.error(archive.validationErrors ?? 'Failed to archive selected repositories');
            // throw error;
          }
        },
      };
    },
  })
  .build();

export const BackupRepositoryDropdownActions = dropdown;
export const BackupRepositoryGroupActions = group;
export const BackupRepositoryInfoActions = info;
