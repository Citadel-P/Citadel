import { startTransition, useCallback } from 'react';
import { Ban, Pause, Play, RotateCcw, Trash, Eye } from 'lucide-react';
import { ContainerView } from '@/api/generated/api.types';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useNavigate } from 'react-router';
import { DockerContainerView } from '@/api/types';
import { IDialogData } from '@/lib/hooks';
import { DropdownAction, DropdownActions } from '@/components/custom/dropdown-menu';

export const TableDropdown = ({
  container,
  hideDetails,
  setDialogData,
}: {
  container: ContainerView | DockerContainerView | undefined;
  setDialogData: (data: IDialogData<ContainerView | DockerContainerView>) => void;
  hideDetails?: boolean;
}) => {
  const navigate = useNavigate();
  const { availableActions, isPending, requestPatch } = useAvailableActions([container as ContainerView]);

  const openDialog = useCallback(() => {
    startTransition(() => {
      if (container) setDialogData({ open: true, currentSelection: [container] });
    });
  }, [setDialogData, container]);

  const items: DropdownAction[] = [
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
      id: 'pause',
      label: 'Pause',
      icon: <Pause className="mr-2 h-3 w-3" />,
      onClick: () => requestPatch('pause'),
      disabled: !availableActions?.canPause || isPending,
    },
    {
      id: 'restart',
      label: 'Restart',
      icon: <RotateCcw className="mr-2 h-3 w-3" />,
      onClick: () => requestPatch('restart'),
      disabled: !availableActions?.canRestart || isPending,
    },
    !hideDetails && {
      id: 'details',
      label: 'View details',
      icon: <Eye className="mr-2 h-3 w-3" />,
      onClick: () => navigate(`../containers/${container?.containerId?.slice(0, 12)}/logs`),
      separatorBefore: true,
    },
    {
      id: 'delete',
      label: 'Delete',
      icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
      onClick: openDialog,
      disabled: !availableActions?.canDelete || isPending,
      danger: true,
      separatorBefore: true,
    },
  ].filter(Boolean) as DropdownAction[];

  return <DropdownActions items={items} />;
};
