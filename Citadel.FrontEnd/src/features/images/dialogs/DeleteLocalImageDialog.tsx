import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from '../ImagesProvider';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useState } from 'react';
import { AppContext } from '@/AppProvider';
import { LoaderCircle } from 'lucide-react';

export const DeleteLocalImageDialog = () => {
  const dialogData = useContextSelector(ImagesContext, (v) => v?.dialogData)!;
  const setDialogData = useContextSelector(ImagesContext, (v) => v?.setDialogData)!;
  const requestDelete = useContextSelector(ImagesContext, (v) => v?.requestDelete)!;
  const deleteIsPending = useContextSelector(ImagesContext, (v) => v?.deleteIsPending) ?? false;
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform)!;

  const registriesId = dialogData.currentSelection?.map((c) => c.id!) ?? [];

  const [noPrune, setNoPrune] = useState(false);
  const [force, setForce] = useState(false);

  const handleDelete = () => {
    requestDelete({ platformId: currentPlatform?.id ?? '', ids: registriesId, force, noPrune });
  };

  return (
    <Dialog open={dialogData.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription></DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          {registriesId.length === 1 && <p>Are you sure you want to delete the selected image?</p>}
          {registriesId.length > 1 && (
            <p>
              Are you sure you want to delete the selected <b>{registriesId.length}</b> images?
            </p>
          )}
          <div className="space-y-2 flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs">
            <div className="space-y-0.5">
              <Label htmlFor="force">Force</Label>
              <p className="text-[0.8rem] text-muted-foreground">
                Remove the image(s) even if it is being used by stopped containers or has other tags
              </p>
            </div>
            <Switch id="force" onCheckedChange={() => setForce((v) => !v)} />
          </div>

          <div className="space-y-2 flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs">
            <div className="space-y-0.5">
              <Label htmlFor="noprune">No Prune</Label>
              <p className="text-[0.8rem] text-muted-foreground">Do not delete untagged parent images</p>
            </div>
            <Switch id="noprune" onCheckedChange={() => setNoPrune((v) => !v)} />
          </div>
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
