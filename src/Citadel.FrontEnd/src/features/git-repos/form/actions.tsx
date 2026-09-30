import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { AuthorizedGitRepositoryView } from '@/api/generated/api.types';
import { syncGitRepositoryAction } from '../actions';

export const { info: GitRepoActions } = createActionsBuilder<AuthorizedGitRepositoryView>()
  .addAction(syncGitRepositoryAction)
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
        ids: [(resource as AuthorizedGitRepositoryView).id],
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
