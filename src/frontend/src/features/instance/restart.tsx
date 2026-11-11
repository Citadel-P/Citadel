import { useState } from 'react';
import { RotateCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useMutate, useRead } from '@/lib/hooks';

export function InstanceRestartButton() {
  const [open, setOpen] = useState(false);
  const [acceptedBoot, setAcceptedBoot] = useState<string>();
  const query = useRead('getInstanceRestart');
  const status = query.data?.data;
  const restart = useMutate('restartInstance');
  const pending = restart.isPending || status?.pending || (!!acceptedBoot && acceptedBoot === status?.bootId);
  const available = !!status?.available && !!status.bootId;
  const confirm = async () => {
    if (!status?.bootId || pending) return;
    try {
      await restart.mutateAsync({ data: { bootId: status.bootId } });
      setAcceptedBoot(status.bootId);
      setOpen(false);
    } catch {
      // The shared mutation hook displays the API error; keep confirmation open for review.
    }
  };
  return (
    <>
      <Button
        size="sm"
        disabled={!available || pending}
        title={
          query.isError
            ? 'Could not load restart availability.'
            : !available
              ? 'In-app restart is unavailable. Restart Core from your host.'
              : undefined
        }
        onClick={() => setOpen(true)}>
        <RotateCw className={pending ? 'size-3.5 animate-spin' : 'size-3.5'} aria-hidden="true" />
        {pending ? 'Restarting…' : 'Restart Core'}
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Restart Core?</DialogTitle>
            <DialogDescription>
              Only saved settings will be applied. Core will disconnect briefly and reconnect automatically. Your
              managed containers will keep running.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)} disabled={restart.isPending}>
              Cancel
            </Button>
            <Button onClick={() => void confirm()} disabled={!available || pending}>
              Restart Core
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
