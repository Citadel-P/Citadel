import React, { startTransition, useCallback } from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { Ban, MoreHorizontal, Pause, Play, RotateCcw, Trash, Eye } from 'lucide-react';
import { ContainerView } from '@/api/_generated';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useNavigate } from 'react-router';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { DockerContainerView } from '@/api/models';
import { IDialogData } from '@/hooks/useDialogState';

export const ContainerDropdownActions: React.FC<{
  container: ContainerView | DockerContainerView | undefined;
  setDialogData: (data: IDialogData<ContainerView | DockerContainerView>) => void;
  hideDetails?: boolean;
}> = ({ container, hideDetails, setDialogData }) => {
  const navigate = useNavigate();

  const { availableActions, isPending, requestPatch } = useAvailableActions([container as ContainerView]);

  // Memoized function to open the delete dialog
  const openDialog = useCallback(() => {
    startTransition(() => {
      if (container) {
        setDialogData({ open: true, currentSelection: [container] });
      }
    });
  }, [setDialogData, container]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        {/* Start Action */}
        <ActionMenuItem
          onClick={() => requestPatch('start')}
          disabled={!availableActions?.canStart || isPending}
          icon={<Play className="mr-2 h-3 w-3" />}
          label="Start"
        />

        {/* Stop Action */}
        <ActionMenuItem
          onClick={() => requestPatch('stop')}
          disabled={!availableActions?.canStop || isPending}
          icon={<Ban className="mr-2 h-3 w-3" />}
          label="Stop"
        />

        {/* Pause Action */}
        <ActionMenuItem
          onClick={() => requestPatch('pause')}
          disabled={!availableActions?.canPause || isPending}
          icon={<Pause className="mr-2 h-3 w-3" />}
          label="Pause"
        />

        {/* Restart Action */}
        <ActionMenuItem
          onClick={() => requestPatch('restart')}
          disabled={!availableActions?.canRestart || isPending}
          icon={<RotateCcw className="mr-2 h-3 w-3" />}
          label="Restart"
        />

        {!hideDetails && (
          <>
            <DropdownMenuSeparator />

            {/* View Details */}
            <ActionMenuItem
              onClick={() => navigate(`../containers/${container?.containerId?.slice(0, 12)}/logs`)}
              disabled={false}
              icon={<Eye className="mr-2 h-3 w-3" />}
              label="View details"
            />
          </>
        )}
        <DropdownMenuSeparator />

        {/* Delete Action */}
        <ActionMenuItem
          onClick={openDialog}
          disabled={!availableActions?.canDelete || isPending}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger hover:text-danger!"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
