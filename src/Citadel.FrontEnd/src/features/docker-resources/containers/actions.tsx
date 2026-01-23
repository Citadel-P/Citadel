import { Ban, Eye, Pause, Play, RotateCcw, StepForward, Trash } from 'lucide-react';
import { ContainerView, ContainerStateStatus, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { formatId } from '@/lib/utils';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';

const useVariables = (resources: ContainerView | ContainerView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.containerId) : [resources.containerId];
const isProcessing = (r: ContainerView) => r.controlState === ResourceControlState.Processing;

export const { dropdown: ContainerDropdownActions, group: ContainerGroupActions } =
  createActionsBuilder<ContainerView>()
    .addAction({
      key: 'start',
      type: 'command',
      icon: Play,
      mutateKey: 'startContainers',
      useVariables,
      canExecute: (r) => {
        const can = (x: ContainerView) =>
          x.state !== ContainerStateStatus.Running &&
          x.state !== ContainerStateStatus.Offline &&
          x.state !== ContainerStateStatus.Paused &&
          !isProcessing(x);
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
        const can = (x: ContainerView) =>
          (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);
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
          const can = (x: ContainerView) => x.state === ContainerStateStatus.Running && !isProcessing(x);
          return Array.isArray(r) ? r.some(can) : can(r);
        },
      },
      secondary: {
        title: 'Resume',
        icon: StepForward,
        mutateKey: 'unpauseContainers',
        useVariables,
        canExecute: (r) => {
          const can = (x: ContainerView) => x.state === ContainerStateStatus.Paused && !isProcessing(x);
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
        const can = (x: ContainerView) =>
          (x.state === ContainerStateStatus.Running || x.state === ContainerStateStatus.Paused) && !isProcessing(x);
        return Array.isArray(r) ? r.some(can) : can(r);
      },
    })
    .addAction({
      key: 'details',
      type: 'command',
      separatorBefore: true,
      icon: Eye,
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const { currentPlatform } = useAppContext();
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
            navigate(`/platforms/${currentPlatform?.id}/containers/${formatId(selected.containerId)}`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteContainers',
      canExecute: (r) => {
        const can = (x: ContainerView) => x.state !== ContainerStateStatus.Offline && !isProcessing(x);
        return Array.isArray(r) ? r.some(can) : can(r);
      },
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Container',
      useVariables: (resources) => {
        const selected = Array.isArray(resources) ? resources : [resources];
        return { force: true, containerIds: selected.map((r) => r.containerId) };
      },
    })
    .build();
