import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useState } from 'react';
import { LoaderCircle } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { ContainersContext } from '../ContainersProvider';

export const DeleteContainerDialog = () => {
  const dialogOpen = useContextSelector(ContainersContext, (v) => v?.dialogData)!;
  const setDialogData = useContextSelector(ContainersContext, (v) => v?.setDialogData)!;
  const isPending = useContextSelector(ContainersContext, (v) => v?.deleteIsPending)!;
  const requestDelete = useContextSelector(ContainersContext, (v) => v?.requestDelete)!;

  const [volume, setVolume] = useState(true);
  const [force, setForce] = useState(true);

  const containersIds = dialogOpen.currentSelection?.map((c) => c.containerId!) ?? [];

  const handleDelete = () => {
    requestDelete({ containersIds, v: volume, force });
  };

  return (
    <Dialog open={dialogOpen.open} onOpenChange={(open) => setDialogData({ open })}>
      <DialogContent className="sm:max-w-[600px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription></DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          {containersIds.length === 1 && <p>Are you sure you want to delete the selected container?</p>}
          {containersIds.length > 1 && (
            <p>
              Are you sure you want to delete the selected <b>{containersIds.length}</b> containers?
            </p>
          )}
          <div className="space-y-2 flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs">
            <div className="space-y-0.5">
              <Label htmlFor="volume">Volume</Label>
              <p className="text-[0.8rem] text-muted-foreground">
                Remove anonymous volumes associated with the container(s).
              </p>
            </div>
            <Switch id="volume" defaultChecked={volume} onCheckedChange={() => setVolume((v) => !v)} />
          </div>
          <div className="space-y-2 flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs">
            <div className="space-y-0.5">
              <Label htmlFor="force">Force</Label>
              <p className="text-[0.8rem] text-muted-foreground">
                If the container(s) is running, kill it before removing it.
              </p>
            </div>
            <Switch defaultChecked={force} id="force" onCheckedChange={() => setForce((v) => !v)} />
          </div>
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
