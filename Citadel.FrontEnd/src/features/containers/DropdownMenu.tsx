import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { Ban, MoreHorizontal, Pause, Play, RotateCcw, Trash, Eye } from 'lucide-react';
import { ContainerInfoView } from '@/api/_generated';
import { useAvailableActions } from './hooks/useAvailableActions';
import { useDialog } from '@/hooks/useDialog';
import { DeleteContainerDialog } from './dialogs/DeleteContainerDialog';
import { useNavigate } from 'react-router';

interface IProps {
  container: ContainerInfoView;
}
const DropdownTableMenu = ({ container }: IProps) => {
  const navigate = useNavigate();
  const deleteDialog = useDialog();
  const { availableActions, isPending, requestPatch } = useAvailableActions([container]);

  return (
    <DeleteContainerDialog
      dialog={deleteDialog}
      containersIds={[container.containerId!]}
      onDelete={() => requestPatch('delete')}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" className="h-8 w-8 p-0">
            <span className="sr-only">Open menu</span>
            <MoreHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
          <DropdownMenuItem
            disabled={!availableActions.canStart || isPending}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card"
            onClick={() => requestPatch('start')}>
            <Play className="mr-2 h-3 w-3" />
            <span>Start</span>
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => requestPatch('stop')}
            disabled={!availableActions.canStop || isPending}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card">
            <Ban className="mr-2 h-3 w-3" />
            <span>Stop</span>
          </DropdownMenuItem>
          <DropdownMenuItem
            onClickCapture={() => requestPatch('pause')}
            disabled={!availableActions.canPause || isPending}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card">
            <Pause className="mr-2 h-3 w-3" />
            <span>Pause</span>
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => requestPatch('restart')}
            disabled={!availableActions.canRestart || isPending}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card">
            <RotateCcw className="mr-2 h-3 w-3" />
            <span>Restart</span>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={() => navigate(`../containers/${container.containerId?.slice(0, 12)}/logs`)}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card">
            <Eye className="mr-2 h-3 w-3" />
            <span>View details</span>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            {...deleteDialog.triggerProps}
            disabled={!availableActions.canDelete || isPending}
            className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold ">
            <Trash className="mr-2 h-3 w-3 text-danger" />
            <span className="text-danger">Delete</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </DeleteContainerDialog>
  );
};

export default DropdownTableMenu;
