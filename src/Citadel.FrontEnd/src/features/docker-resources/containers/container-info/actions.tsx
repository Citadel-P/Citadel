import { Ban, Pause, Play, RotateCcw, StepForward, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DockerContainerView } from '@/api/types';
import { ContainerStateStatus } from '@/api/generated/api.types';

const useVariables = (resources: DockerContainerView | DockerContainerView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

export const { info: ContainerInfoActions } = createActionsBuilder<DockerContainerView>()
  .addAction({
    key: 'start',
    type: 'command',
    icon: Play,
    mutateKey: 'startContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: DockerContainerView) =>
        x.state !== ContainerStateStatus.Running &&
        x.state !== ContainerStateStatus.Offline &&
        x.state !== ContainerStateStatus.Paused;
      return Array.isArray(r) ? r.some(can) : can(r);
    },
  })
  .addAction({
    key: 'stop',
    type: 'command',
    icon: Ban,
    mutateKey: 'stopContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: DockerContainerView) =>
        x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused;
      return Array.isArray(r) ? r.some(can) : can(r);
    },
  })
  .addAction({
    key: 'pauseToggle',
    type: 'toggle',
    primary: {
      title: 'Pause',
      icon: Pause,
      mutateKey: 'pauseContainers',
      useVariables,
      canExecute: (r) => {
        const can = (x: DockerContainerView) => x.state === ContainerStateStatus.Running;
        return Array.isArray(r) ? r.some(can) : can(r);
      },
    },
    secondary: {
      title: 'Resume',
      icon: StepForward,
      mutateKey: 'unpauseContainers',
      useVariables,
      canExecute: (r) => {
        const can = (x: DockerContainerView) => x.state === ContainerStateStatus.Paused;
        return Array.isArray(r) ? r.some(can) : can(r);
      },
    },
  })
  .addAction({
    key: 'restart',
    type: 'command',
    icon: RotateCcw,
    mutateKey: 'restartContainers',
    useVariables,
    canExecute: (r) => {
      const can = (x: DockerContainerView) =>
        x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused;
      return Array.isArray(r) ? r.some(can) : can(r);
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
