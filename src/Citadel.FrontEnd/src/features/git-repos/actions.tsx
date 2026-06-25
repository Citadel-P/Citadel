import { GitRepositoryView } from '@/api/generated/api.types';
import { Pencil, RefreshCw, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { ActionConfig, createActionsBuilder } from '@/components/custom/actions-builder';
import { ResourceControlState } from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

const isProcessing = (resource: GitRepositoryView) => resource.controlState === ResourceControlState.Processing;

export const syncGitRepositoryAction: ActionConfig<GitRepositoryView, 'syncGitRepository'> = {
  key: 'sync',
  type: 'command',
  icon: RefreshCw,
  requiredCapabilities: ['canExecute'],
  useHandler: ({ resources }) => {
    const queryClient = useQueryClient();
    const { mutateAsync, isPending } = useMutate('syncGitRepository');
    const selected = Array.isArray(resources) ? resources[0] : resources;
    const multiSelect = Array.isArray(resources) && resources.length > 1;
    const canExecute = !!selected && !multiSelect && !isProcessing(selected);

    return {
      canExecute,
      isPending,
      run: async () => {
        if (!canExecute || !selected) return;

        await mutateAsync({
          id: selected.id,
        });

        toast.success(`Sync queued for ${selected.name}`);
        queryClient.invalidateQueries({ queryKey: ['listGitRepositories'] });
        queryClient.invalidateQueries({ queryKey: ['getGitRepository', { id: selected.id }] });
        queryClient.invalidateQueries({ queryKey: ['getGitRepositoryConfig', { id: selected.id }] });
      },
    };
  },
};

export const { dropdown: GitRepoDropdownActions, group: GitRepoGroupActions } =
  createActionsBuilder<GitRepositoryView>()
    .addAction(syncGitRepositoryAction)
    .addAction({
      key: 'edit',
      type: 'command',
      icon: Pencil,
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const selected = Array.isArray(resources) ? resources[0] : resources;
        let canExecute = !!selected;
        if (Array.isArray(resources)) {
          canExecute &&= resources.length === 1;
        }
        return {
          canExecute,
          isPending: false,
          run: () => {
            if (!canExecute || !selected) return;
            navigate(`/git-repos/edit/${selected.id}/`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteGitRepositories',
      invalidate: 'listGitRepositories',
      canExecute: () => true,
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'GitRepository',
      useVariables: (resources) => {
        const selected = Array.isArray(resources) ? resources : [resources];
        return {
          ids: selected.map((x) => x.id!),
        };
      },
    })
    .build();
