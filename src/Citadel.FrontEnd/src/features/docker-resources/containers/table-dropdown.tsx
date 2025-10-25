import { Ban, Play, RotateCcw, Trash, Eye, Pause, StepForward } from 'lucide-react';
import { ContainerView } from '@/api/generated/api.types';
import { useAvailableActions } from './hooks/useAvailableActions';
import { createTableDropdown, DropdownAction } from '@/components/custom/dropdown-with-dialog';

export const TableDropdown = ({ container }: { container: ContainerView }) => {
  const { availableActions, isPending, requestPatch } = useAvailableActions([container]);

  return createTableDropdown({
    type: 'Container',
    actions: ({ navigate, openDialog }) => {
      const actions: DropdownAction[] = [
        {
          id: 'start',
          label: 'Start',
          icon: <Play className="mr-2 h-3 w-3" />,
          onClick: () => requestPatch('start'),
          disabled: !availableActions?.canStart || isPending,
        },
        {
          id: 'stop',
          label: 'Stop',
          icon: <Ban className="mr-2 h-3 w-3" />,
          onClick: () => requestPatch('stop'),
          disabled: !availableActions?.canStop || isPending,
        },
        {
          id: availableActions?.canUnpause ? 'unpause' : 'pause',
          label: availableActions?.canUnpause ? 'Resume' : 'Pause',
          icon: availableActions?.canUnpause ? (
            <StepForward className="mr-2 h-3 w-3" />
          ) : (
            <Pause className="mr-2 h-3 w-3" />
          ),
          onClick: () => requestPatch(availableActions?.canUnpause ? 'unpause' : 'pause'),
          disabled: availableActions?.canUnpause
            ? !availableActions.canUnpause || isPending
            : !availableActions?.canPause || isPending,
        },
        {
          id: 'restart',
          label: 'Restart',
          icon: <RotateCcw className="mr-2 h-3 w-3" />,
          onClick: () => requestPatch('restart'),
          disabled: !availableActions?.canRestart || isPending,
        },
        {
          id: 'details',
          label: 'View details',
          icon: <Eye className="mr-2 h-3 w-3" />,
          onClick: () => navigate(`../containers/${container.containerId?.slice(0, 12)}/logs`),
          separatorBefore: true,
        },
      ];

      actions.push({
        id: 'delete',
        label: 'Delete',
        icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
        onClick: () => openDialog(container),
        disabled: !availableActions?.canDelete || isPending,
        danger: true,
        separatorBefore: true,
      });

      return actions;
    },
  });
};
