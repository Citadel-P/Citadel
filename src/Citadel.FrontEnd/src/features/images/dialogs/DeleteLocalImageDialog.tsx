import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useState } from 'react';
import { LoaderCircle } from 'lucide-react';
import { useAppContext } from '@/AppContext';
import { SwitchSection } from '@/components/ui/SwitchSection';
import { IDialogData } from '@/hooks/useDialogState';
import { DeleteImagesRequest, ImageView } from '@/api/_generated';

export const DeleteLocalImageDialog = ({
  dialogData,
  setDialogData,
  requestDelete,
  deleteIsPending,
}: {
  dialogData: IDialogData<ImageView>;
  setDialogData: (data: IDialogData<ImageView>) => void;
  requestDelete: (request: DeleteImagesRequest) => void;
  deleteIsPending: boolean;
}) => {
  const { currentPlatform } = useAppContext();

  const imagesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const [noPrune, setNoPrune] = useState(false);
  const [force, setForce] = useState(false);

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: imagesId, force, noPrune });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Confirm Deletion</DialogTitle>
          <DialogDescription>
            {imagesId.length === 1 ? (
              'Are you sure you want to delete the selected image?'
            ) : (
              <>
                You&apos;re about to delete <span className="font-medium text-foreground">{imagesId.length}</span>{' '}
                images. This action is permanent. Do you want to continue?
              </>
            )}
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
