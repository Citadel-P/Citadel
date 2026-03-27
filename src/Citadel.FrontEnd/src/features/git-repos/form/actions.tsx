import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { GitRepositoryView } from '@/api/generated/api.types';

export const { info: GitRepoActions } = createActionsBuilder<GitRepositoryView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteGitRepositories',
    confirm: true,
    destructive: true,
    resourceType: 'GitRepository',
    canExecute: () => true,
    useVariables: (resource) => {
      return {
        ids: [(resource as GitRepositoryView).id],
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/git-repos`);
      };
    },
  })
  .build();
