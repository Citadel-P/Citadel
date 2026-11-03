import { useEffect, useState } from 'react';
import { useAtom, useStore } from 'jotai';
import { useQueryClient } from '@tanstack/react-query';
import { RefreshCw } from 'lucide-react';
import { toast } from 'sonner';
import { useMutate, useRead } from '@/lib/hooks';
import { pendingInstanceUpdateAtom } from '@/lib/instance-update-state';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import type { UpdateReviewView, UpdateStatusView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { AlertMessage } from '@/components/custom/alert-message';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function InstallUpdate({ status }: { status: UpdateStatusView }) {
  const queryClient = useQueryClient();
  const [, remember] = useAtom(pendingInstanceUpdateAtom);
  const [review, setReview] = useState<UpdateReviewView>();
  const [requestId, setRequestId] = useState('');
  const prepare = useMutate('reviewInstanceUpdate', {
    onSuccess: ({ data }) => {
      setReview(data);
      setRequestId(crypto.randomUUID());
    },
  });
  const approve = useMutate('approveInstanceUpdate', {
    onSuccess: async ({ data }) => {
      try {
        remember({ instanceId: status.instanceId, id: data.id, restartUntil: Date.now() + 15 * 60_000 });
      } catch {
        // A browser storage failure cannot undo a server-approved operation.
      }
      await queryClient.cancelQueries({ queryKey: ['getInstanceUpdates'] });
      queryClient.setQueryData(['getInstanceUpdates', {}], { data: { ...status, operation: data } });
      setReview(undefined);
      toast.success('Update approved', { description: 'Citadel will prepare a recovery checkpoint and restart.' });
    },
  });
  if (!status.available || status.operation || status.installationUnavailableReason) return null;
  return (
    <>
      <Button
        size="sm"
        disabled={prepare.isPending}
        onClick={() => prepare.mutate({ data: { version: status.available!.version } })}>
        <RefreshCw className={prepare.isPending ? 'size-4 animate-spin' : 'size-4'} />
        {prepare.isPending ? 'Preparing review…' : 'Review update'}
      </Button>
      <Dialog
        open={!!review}
        onOpenChange={(open) => {
          if (!open && !approve.isPending) setReview(undefined);
        }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Update Citadel</DialogTitle>
            <DialogDescription>
              Citadel {review?.installedVersion} → {review?.targetVersion}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 text-sm">
            <p>
              Citadel will finish active work, create a recovery checkpoint, then restart. The portal will be
              temporarily unavailable.
            </p>
            <p className="text-muted-foreground">
              Your managed workloads remain running. If verification fails, Citadel stays in maintenance for operator
              recovery.
            </p>
            {approve.isError && (
              <p role="alert">
                Approval could not be confirmed. Retrying uses the same request and will not start a second update.
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" disabled={approve.isPending} onClick={() => setReview(undefined)}>
              Cancel
            </Button>
            <Button
              disabled={approve.isPending || !review}
              onClick={() => approve.mutate({ data: { reviewId: review!.id, requestId } })}>
              {approve.isPending ? 'Approving…' : 'Update and restart'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

const labels: Record<string, string> = {
  Preparing: 'Preparing update',
  Draining: 'Waiting for active work to finish',
  BackingUp: 'Creating recovery checkpoint',
  ReplacingCore: 'Restarting Citadel',
  Verifying: 'Verifying the new version',
  Succeeded: 'Update completed',
  Failed: 'Update failed — operator recovery required',
  Postponed: 'Update postponed',
  RecoveryRequired: 'Operator recovery required',
};
const terminal = new Set(['Succeeded', 'Failed', 'Postponed', 'RecoveryRequired']);

export function UpdateProgress({ status }: { status: UpdateStatusView }) {
  const [pending, remember] = useAtom(pendingInstanceUpdateAtom);
  const store = useStore();
  const { liveConnectionState } = useRealtimeContext();
  const id = pending?.instanceId === status.instanceId ? pending.id : undefined;
  const result = useRead(
    'getInstanceUpdateOperation',
    { id: id ?? '' },
    { enabled: !!id && !status.operation, retry: false },
  );
  const operation = status.operation ?? result.data?.data;
  // A committed outcome still owns maintenance until host cleanup is verified.
  const finishing = status.operation?.phase === 'Succeeded';
  const completed = !status.operation && operation?.phase === 'Succeeded';
  useEffect(() => {
    try {
      if (status.operation && (!terminal.has(status.operation.phase) || finishing)) {
        remember((previous) => ({
          ...(previous?.instanceId === status.instanceId && previous.id === status.operation!.id ? previous : {}),
          instanceId: status.instanceId,
          id: status.operation!.id,
          restartUntil:
            previous?.instanceId === status.instanceId && previous.id === status.operation!.id && previous.restartUntil
              ? previous.restartUntil
              : ['Draining', 'BackingUp', 'ReplacingCore', 'Verifying', 'Succeeded'].includes(status.operation!.phase)
                ? Date.now() + 15 * 60_000
                : undefined,
        }));
      } else if (operation && terminal.has(operation.phase)) {
        remember((previous) =>
          previous?.instanceId === status.instanceId && previous.id === operation.id && previous.restartUntil
            ? { ...previous, restartUntil: undefined }
            : previous,
        );
      }
    } catch {
      // The API remains authoritative when browser storage is unavailable.
    }
  }, [status.operation, status.instanceId, operation, finishing, remember]);

  useEffect(() => {
    if (!completed || liveConnectionState !== 'connected' || status.installedVersion !== operation?.version) return;
    // Read the current atom so repeated effects cannot request a second reload.
    const saved = store.get(pendingInstanceUpdateAtom);
    if (saved?.instanceId !== status.instanceId || saved.id !== operation.id || saved.frontendReloaded) return;
    try {
      // Persist before navigation. If session storage is unavailable, leave the
      // manual reload action visible rather than risking a reload loop.
      store.set(pendingInstanceUpdateAtom, { ...saved, restartUntil: undefined, frontendReloaded: true });
      location.reload();
    } catch {
      // Completion remains visible, with an explicit reload action below.
    }
  }, [completed, liveConnectionState, status.instanceId, status.installedVersion, operation, store]);

  if (!operation) return null;
  const disconnected = liveConnectionState !== 'connected' && (!terminal.has(operation.phase) || finishing);
  const failed = ['Failed', 'RecoveryRequired'].includes(operation.phase);
  return (
    <AlertMessage
      type={failed ? 'warning' : 'info'}
      className="my-0"
      title={finishing ? 'Finishing update' : (labels[operation.phase] ?? 'Update in progress')}
      date={operation.updatedAt}
      action={
        completed && liveConnectionState === 'connected' ? (
          <Button size="sm" variant="outline" onClick={() => location.reload()}>
            <RefreshCw className="size-4" />
            Reload portal
          </Button>
        ) : undefined
      }>
      {disconnected
        ? 'Waiting for Citadel to reconnect. Progress will resume when the server returns.'
        : finishing
          ? 'The new version is verified. Citadel is completing cleanup before reopening the portal.'
          : terminal.has(operation.phase)
            ? `Target version: Citadel ${operation.version}.`
            : 'You can leave this page. The server owns this operation; closing your browser will not cancel it.'}
    </AlertMessage>
  );
}
