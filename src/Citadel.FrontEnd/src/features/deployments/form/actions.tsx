import { RefreshCw, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentView } from '@/api/generated/api.types';
import { deployAction, pauseAction, startAction, stopAction, useVariables } from '../actions';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { getDeploymentUpdateCheckDisabledReason, hasDeploymentUpdateAvailable } from '../update-status';

export const { info: DeploymentActions } = createActionsBuilder<DeploymentView>()
  .addAction(deployAction)
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction({
    key: 'checkUpdates',
    title: 'Check for updates',
    type: 'command',
    icon: RefreshCw,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const queryClient = useQueryClient();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;
      const { mutateAsync, isPending } = useMutate('checkDeploymentUpdates');
      const disabledReason = multiSelect
        ? 'Select one deployment to check for updates.'
        : getDeploymentUpdateCheckDisabledReason(selected);
      const canExecute = disabledReason === undefined;

      return {
        canExecute,
        disabledReason,
        isPending,
        run: async () => {
          if (!selected || !canExecute) return;

          try {
            const result = await mutateAsync({ deploymentId: selected.id });
            if (hasDeploymentUpdateAvailable(result.data)) {
              toast.info('Update available', { description: 'A newer image digest is available.' });
            } else {
              toast.success('Deployment is up to date', {
                description: 'No newer image digest was found.',
              });
            }

            await Promise.all([
              queryClient.invalidateQueries({ queryKey: ['getDeployment'] }),
              queryClient.invalidateQueries({ queryKey: ['listDeployments'] }),
            ]);
          } catch {
            // Nope
          }
        },
      };
    },
  })
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
