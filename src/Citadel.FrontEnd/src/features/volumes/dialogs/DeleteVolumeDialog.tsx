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
import { useVolumesContext } from '../VolumesContext';
import { useState } from 'react';
import { SwitchSection } from '@/components/ui/SwitchSection';

export const DeleteVolumeDialog = () => {
  const { dialogData, setDialogData, requestDelete, deleteIsPending } = useVolumesContext();
  const { currentPlatform } = useAppContext();
  const [forceDelete, setForceDelete] = useState(false);

  const volumesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', names: volumesId, force: forceDelete });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription>
            {volumesId.length === 1
              ? 'Are you sure you want to delete the selected volume?'
              : `Are you sure you want to delete the selected ${volumesId.length} volumes?`}
          </DialogDescription>
        </DialogHeader>
        <SwitchSection
          id="force"
          label="Force"
          description="Force the removal of the volume."
          checked={forceDelete}
          onToggle={() => setForceDelete(!forceDelete)}
        />
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
