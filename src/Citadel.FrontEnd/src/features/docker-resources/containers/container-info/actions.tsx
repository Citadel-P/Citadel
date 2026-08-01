import { FolderInput, PackagePlus, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { containsSystemContainer, createContainerActions, isAdoptableContainer, isImportableStack } from '../actions';
import { ResourceControlState } from '@/api/generated/api.types';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';

const useVariables = (resources: ContainerDetailsView | ContainerDetailsView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

const { startAction, stopAction, pauseAction, restartAction } = createContainerActions(useVariables);

export const { info: ContainerInfoActions } = createActionsBuilder<ContainerDetailsView>()
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction(restartAction)
  .addAction({
    key: 'adopt',
    title: 'Adopt Container',
    type: 'command',
    icon: PackagePlus,
    requiredCapabilities: ['canInspect'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const canExecute =
        !!selected && isAdoptableContainer(selected) && selected.controlState !== ResourceControlState.Processing;

      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !selected) return;
          navigate(`/deployments/add?adoptFrom=${selected.resourceId}`);
        },
      };
    },
  })
  .addAction({
    key: 'importStack',
    title: 'Import Stack',
    type: 'command',
    icon: FolderInput,
    requiredCapabilities: ['canInspect'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const canExecute =
        !!selected && isImportableStack(selected) && selected.controlState !== ResourceControlState.Processing;

      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !selected?.stack) return;
          const query = new URLSearchParams({
            importPlatform: selected.platformId,
            importProject: selected.stack,
          });
          navigate(`/stacks/add?${query.toString()}`);
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteContainers',
    confirm: true,
    destructive: true,
    resourceType: 'Container',
    canExecute: (resource) => {
      const selected = Array.isArray(resource) ? resource : [resource];
      return !containsSystemContainer(selected);
    },
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

export const getContainerManagementAction = (resource: ContainerDetailsView) => {
  if (isAdoptableContainer(resource)) return ContainerInfoActions.adopt;
  if (isImportableStack(resource)) return ContainerInfoActions.importStack;
  return undefined;
};
