import { GitRepositoryView } from '@/api/generated/api.types';
import { Pencil, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';

export const { dropdown: GitRepoDropdownActions, group: GitRepoGroupActions } =
  createActionsBuilder<GitRepositoryView>()
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
