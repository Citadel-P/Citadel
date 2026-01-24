import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentView } from '@/api/generated/api.types';
import { deployAction, pauseAction, startAction, stopAction, useVariables } from '../actions';

export const { info: DeploymentActions } = createActionsBuilder<DeploymentView>()
  .addAction(deployAction)
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteDeployments',
    confirm: true,
    destructive: true,
    resourceType: 'Deployment',
    useVariables,
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/deployments`);
      };
    },
  })
  .build();
