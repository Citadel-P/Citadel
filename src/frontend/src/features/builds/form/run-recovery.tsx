import { useId, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import type { BuildRunView } from '@/api/generated/api.types';
import { useMutate, useRead } from '@/lib/hooks';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function BuildRunRecovery({ run }: { run: BuildRunView }) {
  const profile = useRead('getCurrentProfile');
  const administrator = profile.data?.data.authorization.isAdministrator === true;
  const recovery = useRead('getBuildRunRecovery', { id: run.id }, { enabled: administrator });
  const resolve = useMutate('resolveBuildRunCleanup');
  const queryClient = useQueryClient();
  const id = useId();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  const reservation = recovery.data?.data.reservation;

  async function submit() {
    if (!reservation) return;
    try {
      await resolve.mutateAsync({
        id: run.id,
        data: { attemptId: reservation.attemptId, reason: reason.trim(), acknowledged },
      });
      setOpen(false);
      toast.success('Retained capacity released. The build result is unchanged.');
      await Promise.all(
        [
          ['getBuildRunRecovery', { id: run.id }],
          ['getBuildRun', { id: run.id }],
          ['getBuildRunLogs', { id: run.id }],
          ['listBuildRuns'],
          ['getBuildProject', { id: run.buildProjectId }],
        ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
      );
    } catch {
      // Keep the confirmation open; the request error is displayed below.
    }
  }

  if (!administrator) return null;
  if (recovery.isError)
    return (
      <AlertMessage
        type="warning"
        action={
          <Button size="sm" variant="outline" onClick={() => void recovery.refetch()}>
            Retry
          </Button>
        }>
        Could not check retained build capacity.
      </AlertMessage>
    );
  if (!reservation) return null;

  return (
    <>
      <AlertMessage
        type="warning"
        title="Cleanup pending"
        action={
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              setReason('');
              setAcknowledged(false);
              resolve.reset();
              setOpen(true);
            }}>
            Resolve cleanup
          </Button>
        }>
        This attempt still holds build capacity.
      </AlertMessage>
      <Dialog
        open={open}
        onOpenChange={(value) => {
          if (!resolve.isPending) setOpen(value);
        }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Resolve build cleanup</DialogTitle>
            <DialogDescription>
              Check the original host and stop any remaining build or registry pushes for this attempt. Once other
              builds on the host finish, restart the original Agent. Citadel verifies the Agent changed and still uses
              the same Docker daemon; your confirmation records that the old work has stopped.
            </DialogDescription>
          </DialogHeader>
          <dl className="grid gap-1 text-sm">
            <dt className="text-muted-foreground">Host</dt>
            <dd>{run.platformSnapshot?.name ?? 'Original build host'}</dd>
            <dt className="text-muted-foreground">Attempt</dt>
            <dd className="break-all font-mono text-xs">{reservation.attemptId}</dd>
            <dt className="text-muted-foreground">Docker daemon</dt>
            <dd className="break-all font-mono text-xs">{reservation.daemonId}</dd>
          </dl>
          <div className="space-y-2">
            <Label htmlFor={`${id}-reason`}>Resolution notes</Label>
            <Textarea
              id={`${id}-reason`}
              value={reason}
              maxLength={1000}
              disabled={resolve.isPending}
              onChange={(event) => setReason(event.target.value)}
              placeholder="What did you verify or stop on the host?"
            />
          </div>
          <div className="flex items-start gap-2">
            <Checkbox
              id={`${id}-confirm`}
              checked={acknowledged}
              disabled={resolve.isPending}
              onCheckedChange={(checked) => setAcknowledged(checked === true)}
            />
            <Label htmlFor={`${id}-confirm`} className="text-sm leading-relaxed">
              I verified that this attempt’s build and pushes have stopped and restarted the original Agent.
            </Label>
          </div>
          <p className="text-sm text-muted-foreground">
            Releases this reservation and records your acknowledgment in Activities and run logs. The run result is
            preserved; the build will not be retried.
          </p>
          {resolve.isError && (
            <p role="alert" className="text-sm text-destructive">
              {resolve.error.message}
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" disabled={resolve.isPending} onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button disabled={resolve.isPending || !acknowledged || !reason.trim()} onClick={() => void submit()}>
              {resolve.isPending ? 'Resolving…' : 'Release capacity'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
