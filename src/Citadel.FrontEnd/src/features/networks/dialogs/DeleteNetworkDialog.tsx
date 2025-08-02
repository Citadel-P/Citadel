import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { LoaderCircle } from 'lucide-react';
import { useAppContext } from '@/AppContext';
import { useNetworksContext } from '../NetworksContext';

export const DeleteNetworkDialog = () => {
  const { dialogData, setDialogData, requestDelete, deleteIsPending } = useNetworksContext();

  const { currentPlatform } = useAppContext();

  const networksId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: networksId });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Confirm Deletion</DialogTitle>
          <DialogDescription>
            {networksId.length === 1 ? (
              'Are you sure you want to delete the selected network?'
            ) : (
              <>
                You&apos;re about to delete <span className="font-medium text-foreground">{networksId.length}</span>{' '}
                networks. This action is permanent. Do you want to continue?
              </>
            )}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <div className="flex items-center justify-end">
            <button
              type="button"
              onClick={() => setDialogData({ open: false })}
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
