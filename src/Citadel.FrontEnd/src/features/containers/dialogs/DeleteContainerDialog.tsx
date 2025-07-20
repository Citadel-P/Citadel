import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useReducer } from 'react';
import { LoaderCircle } from 'lucide-react';
import { useContainersContext } from '../ContainersProvider';
import { SwitchSection } from '@/components/ui/SwitchSection';

// Reducer for managing volume and force states
const toggleReducer = (state: boolean, _action: void) => !state;

export const DeleteContainerDialog = () => {
  const { deleteIsPending: isPending, requestDelete, dialogData, setDialogData } = useContainersContext();
  const [volume, toggleVolume] = useReducer(toggleReducer, true);
  const [force, toggleForce] = useReducer(toggleReducer, true);

  const containerIds = dialogData.currentSelection?.map((c) => c.containerId!) ?? [];

  const handleDelete = () => {
    requestDelete({ containerIds, v: volume, force });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription>
            {containerIds.length === 1
              ? 'Are you sure you want to delete the selected container?'
              : `Are you sure you want to delete the selected ${containerIds.length} containers?`}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <SwitchSection
            id="volume"
            label="Volume"
            description="Remove anonymous volumes associated with the container(s)."
            checked={volume}
            onToggle={toggleVolume}
          />
          <SwitchSection
            id="force"
            label="Force"
            description="If the container(s) is running, kill it before removing it."
            checked={force}
            onToggle={toggleForce}
          />
        </div>
        <DialogFooter>
          <div className="flex items-center justify-end">
            <button
              onClick={() => setDialogData({ open: false })}
              type="button"
              className="text-foreground bg-secondary hover:bg-secondary/80 font-medium rounded-sm text-sm px-2 py-2">
              Cancel
            </button>
            <button
              type="button"
              disabled={isPending}
              onClick={handleDelete}
              className="ml-2 bg-danger hover:bg-danger/85 text-background font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
              Delete
              {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
            </button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
