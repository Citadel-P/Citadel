import { useEffect } from 'react';
import { useAtom, useStore } from 'jotai';
import { ArrowRight, RefreshCw, ShieldCheck } from 'lucide-react';
import { toast } from 'sonner';
import { useRead } from '@/lib/hooks';
import { pendingInstanceUpdateAtom } from '@/lib/instance-update-state';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import type { UpdateReviewView, UpdateStatusView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { AlertMessage } from '@/components/custom/alert-message';

export function CoreReviewDetails({ review }: { review: UpdateReviewView }) {
  return (
    <>
      <div className="grid grid-cols-[minmax(0,1fr)_1rem_minmax(0,1fr)] items-center gap-3 rounded-md border bg-muted/30 p-4 text-center text-sm">
        <dl className="min-w-0 space-y-1">
          <dt className="text-xs text-muted-foreground">Installed</dt>
          <dd className="wrap-anywhere font-medium">{review.installedVersion}</dd>
        </dl>
        <ArrowRight aria-hidden="true" className="size-4 text-muted-foreground" />
        <dl className="min-w-0 space-y-1">
          <dt className="text-xs text-muted-foreground">New version</dt>
          <dd className="wrap-anywhere font-medium">{review.targetVersion}</dd>
        </dl>
      </div>
      <div className="space-y-4 text-sm">
        <div className="flex items-start gap-3">
          <ShieldCheck aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="space-y-1">
            <p className="font-medium">Recovery checkpoint</p>
            <p className="leading-relaxed text-muted-foreground">
              Citadel waits for active work to finish, then creates a checkpoint before updating.
            </p>
          </div>
        </div>
        <div className="flex items-start gap-3">
          <RefreshCw aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="space-y-1">
            <p className="font-medium">Portal restart</p>
            <p className="leading-relaxed text-muted-foreground">
              The portal will be temporarily unavailable and reconnect automatically. Your managed workloads keep
              running.
            </p>
          </div>
        </div>
      </div>
      <p className="text-xs leading-relaxed text-muted-foreground">
        If verification fails, Citadel remains in maintenance and requires manual recovery.
      </p>
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
  const id = pending && pending.instanceId === status.instanceId ? pending.id : undefined;
  const result = useRead(
    'getInstanceUpdateOperation',
    { id: id ?? '' },
    { enabled: !!id && !status.operation, retry: false },
  );
  const operation = status.operation ?? (id ? result.data?.data : undefined);
  // A committed outcome still owns maintenance until host cleanup is verified.
  const finishing = status.operation?.phase === 'Succeeded';
  const completed = !status.operation && operation?.phase === 'Succeeded';
  useEffect(() => {
    try {
      if (status.operation && (!terminal.has(status.operation.phase) || finishing)) {
        remember((previous) => ({
          ...(previous && previous.instanceId === status.instanceId && previous.id === status.operation!.id
            ? previous
            : {}),
          instanceId: status.instanceId,
          id: status.operation!.id,
          restartUntil:
            previous &&
            previous.instanceId === status.instanceId &&
            previous.id === status.operation!.id &&
            previous.restartUntil
              ? previous.restartUntil
              : ['Draining', 'BackingUp', 'ReplacingCore', 'Verifying', 'Succeeded'].includes(status.operation!.phase)
                ? Date.now() + 15 * 60_000
                : undefined,
        }));
      } else if (operation && terminal.has(operation.phase)) {
        remember((previous) =>
          previous && previous.instanceId === status.instanceId && previous.id === operation.id && previous.restartUntil
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
    if (!saved || saved.instanceId !== status.instanceId || saved.id !== operation.id) return;
    try {
      if (saved.frontendReloaded) {
        store.set(pendingInstanceUpdateAtom, null);
        toast.success('Citadel updated successfully', {
          id: `instance-update-${status.instanceId}-${operation.id}`,
          description: `Version ${operation.version} is now running.`,
        });
        return;
      }
      // Persist before navigation. If session storage is unavailable, leave the
      // manual reload action visible rather than risking a reload loop.
      store.set(pendingInstanceUpdateAtom, { ...saved, restartUntil: undefined, frontendReloaded: true });
      location.reload();
    } catch {
      // Completion remains visible, with an explicit reload action below.
    }
  }, [completed, liveConnectionState, status.instanceId, status.installedVersion, operation, store]);

  useEffect(() => {
    if (!operation || !['Failed', 'RecoveryRequired'].includes(operation.phase)) return;
    const saved = store.get(pendingInstanceUpdateAtom);
    const sameOperation = saved?.instanceId === status.instanceId && saved?.id === operation.id;
    if (sameOperation && saved.failureNotified) return;
    toast.error('Citadel update failed', {
      id: `instance-update-${status.instanceId}-${operation.id}`,
      description: status.operation
        ? `The update to ${operation.version} needs manual recovery. Check Activities for details.`
        : 'The update was not installed. Citadel is running the original version. Check Activities for details.',
    });
    try {
      store.set(pendingInstanceUpdateAtom, {
        ...(sameOperation ? saved : {}),
        instanceId: status.instanceId,
        id: operation.id,
        restartUntil: undefined,
        failureNotified: true,
      });
    } catch {
      // A storage failure must not suppress the error or hide recovery state.
    }
  }, [operation, status.instanceId, status.operation, store]);

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
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              try {
                remember((previous) =>
                  previous && previous.instanceId === status.instanceId && previous.id === operation.id
                    ? { ...previous, restartUntil: undefined, frontendReloaded: true }
                    : previous,
                );
              } catch {
                // An explicit reload remains available when storage is unavailable.
              }
              location.reload();
            }}>
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
