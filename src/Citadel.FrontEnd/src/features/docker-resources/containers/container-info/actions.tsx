import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DockerContainerView } from '@/api/types';
import { createContainerActions } from '../actions';

const useVariables = (resources: DockerContainerView | DockerContainerView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

const { startAction, stopAction, pauseAction, restartAction } = createContainerActions(useVariables);

export const { info: ContainerInfoActions } = createActionsBuilder<DockerContainerView>()
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction(restartAction)
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteContainers',
    confirm: true,
    destructive: true,
    resourceType: 'Container',
    canExecute: () => true,
    useVariables: (resource) => {
      const selected = Array.isArray(resource) ? resource : [resource];
      return { force: true, containerIds: selected.map((r) => r.id) };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();
      return () => {
        navigate(`/platforms/${currentPlatform?.id}/containers`);
      };
    },
  })
  .build();
