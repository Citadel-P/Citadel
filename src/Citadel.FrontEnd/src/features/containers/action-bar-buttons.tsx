import { Play, Pause, RotateCcw, Ban, Trash, StepForward } from 'lucide-react';
import { useAvailableActions } from './hooks/useAvailableActions';
import { ContainerView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { IDialogData } from '@/lib/hooks';
import { ActionButtonConfig, ActionButtons } from '@/components/custom/action-bar';

export const ActionBarButtons = ({
  selectedContainers,
  setDialogData,
}: {
  selectedContainers: ContainerView[] | DockerContainerView[] | undefined;
  setDialogData: (_: IDialogData<ContainerView | DockerContainerView>) => void;
}) => {
  const { availableActions, isPending, requestPatch } = useAvailableActions(selectedContainers);

  const buttons: ActionButtonConfig[] = [
    {
      id: 'start',
      icon: Play,
      label: 'Start',
      onClick: () => requestPatch('start'),
      disabled: !availableActions?.canStart || isPending,
      ariaLabel: 'Start selected containers',
    },
    {
      id: 'stop',
      icon: Ban,
      label: 'Stop',
      onClick: () => requestPatch('stop'),
      disabled: !availableActions?.canStop || isPending,
      ariaLabel: 'Stop selected containers',
    },
    {
      id: availableActions?.canUnpause ? 'unpause' : 'pause',
      icon: availableActions?.canUnpause ? StepForward : Pause,
      label: availableActions?.canUnpause ? 'Resume' : 'Pause',
      onClick: () => requestPatch(availableActions?.canUnpause ? 'unpause' : 'pause'),
      disabled: availableActions?.canUnpause
        ? !availableActions.canUnpause || isPending
        : !availableActions?.canPause || isPending,
      ariaLabel: availableActions?.canUnpause ? 'Resume selected containers' : 'Pause selected containers',
    },
    {
      id: 'restart',
      icon: RotateCcw,
      label: 'Restart',
      onClick: () => requestPatch('restart'),
      disabled: !availableActions?.canRestart || isPending,
      ariaLabel: 'Restart selected containers',
    },
    {
      id: 'delete',
      icon: Trash,
      label: 'Delete',
      onClick: () => setDialogData({ open: true, currentSelection: selectedContainers }),
      disabled: !availableActions?.canDelete || isPending,
      ariaLabel: 'Delete selected containers',
      variant: 'danger' as const,
    },
  ];

  return <ActionButtons buttons={buttons} />;
};
