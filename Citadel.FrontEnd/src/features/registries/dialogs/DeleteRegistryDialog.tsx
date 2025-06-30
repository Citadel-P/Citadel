import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useRegistriesContext } from '../RegistriesProvider';
import { LoaderCircle } from 'lucide-react';

export const DeleteRegistryDialog = () => {
  const { requestDelete, dialogData, setDialogData, deleteIsPending: isPending } = useRegistriesContext();
  const registriesId = dialogData.currentSelection?.map((r) => r.id!) ?? [];

  const handleDelete = () => {
    requestDelete(registriesId);
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription></DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          {registriesId.length === 1 && <p>Are you sure you want to delete the selected registry?</p>}
          {registriesId.length > 1 && (
            <p>
              Are you sure you want to delete the selected <b>{registriesId.length}</b> registries?
            </p>
          )}
        </div>
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
