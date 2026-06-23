import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { StackView } from '@/api/generated/api.types';
import { deployAction, pauseAction, startAction, stopAction, syncAction, useVariables } from '../actions';

export const { info: StackActions } = createActionsBuilder<StackView>()
  .addAction(deployAction)
  .addAction(syncAction)
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteStacks',
    confirm: true,
    destructive: true,
    resourceType: 'Stack',
    useVariables,
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/stacks`);
      };
    },
  })
  .build();
