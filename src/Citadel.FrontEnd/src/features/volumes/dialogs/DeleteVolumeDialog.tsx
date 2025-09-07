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
import { useState } from 'react';
import { SwitchSection } from '@/components/ui/SwitchSection';
import { IDialogData } from '@/hooks/useDialogState';
import { DeleteVolumesInput, DockerVolumeResult } from '@/api/_generated';

export const DeleteVolumeDialog = ({
  dialogData,
  setDialogData,
  requestDelete,
  deleteIsPending,
}: {
  dialogData: IDialogData<DockerVolumeResult>;
  setDialogData: (data: IDialogData<DockerVolumeResult>) => void;
  requestDelete: (request: DeleteVolumesInput) => void;
  deleteIsPending: boolean;
}) => {
  const { currentPlatform } = useAppContext();
  const [forceDelete, setForceDelete] = useState(false);

  const volumesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', names: volumesId, force: forceDelete });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Confirm Deletion</DialogTitle>
          <DialogDescription>
            {volumesId.length === 1 ? (
              'Are you sure you want to delete the selected volume?'
            ) : (
              <>
                You&apos;re about to delete <span className="font-medium text-foreground">{volumesId.length}</span>{' '}
                volumes. This action is permanent. Do you want to continue?
              </>
            )}
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
