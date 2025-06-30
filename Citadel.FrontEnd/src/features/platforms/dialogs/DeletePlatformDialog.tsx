import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { LoaderCircle } from 'lucide-react';
import { usePlatformsContext } from '../PlatformsProvider';

export const DeletePlatformDialog = () => {
  const { dialogData, setDialogData, deleteIsPending, requestDelete } = usePlatformsContext();
  const handleDelete = () => {
    if (dialogData.platform) requestDelete(dialogData.platform.id);
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete the <span className="font-medium">{dialogData.platform?.name}</span>{' '}
            platform?
          </DialogDescription>
        </DialogHeader>

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
              disabled={deleteIsPending}
              onClick={handleDelete}
              className="ml-2 bg-danger hover:bg-danger/85 text-background font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
              Delete
              {deleteIsPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
            </button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
