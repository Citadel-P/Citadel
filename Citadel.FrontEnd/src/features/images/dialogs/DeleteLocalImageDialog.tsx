import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useImagesContext } from '../ImagesProvider';
import { useState } from 'react';
import { LoaderCircle } from 'lucide-react';
import { useAppContext } from '@/AppProvider';
import { SwitchSection } from '@/components/ui/SwitchSection';

export const DeleteLocalImageDialog = () => {
  const { currentPlatform } = useAppContext();
  const { dialogData, setDialogData, requestDelete, deleteIsPending } = useImagesContext();

  const imagesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const [noPrune, setNoPrune] = useState(false);
  const [force, setForce] = useState(false);

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: imagesId, force, noPrune });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription>
            {imagesId.length === 1
              ? 'Are you sure you want to delete the selected image?'
              : `Are you sure you want to delete the selected ${imagesId.length} images?`}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <SwitchSection
            id="force"
            label="Force"
            description="Remove the image(s) even if it is being used by stopped containers or has other tags."
            checked={force}
            onToggle={() => setForce((v) => !v)}
          />
          <SwitchSection
            id="noprune"
            label="No Prune"
            description="Do not delete untagged parent images."
            checked={noPrune}
            onToggle={() => setNoPrune((v) => !v)}
          />
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
