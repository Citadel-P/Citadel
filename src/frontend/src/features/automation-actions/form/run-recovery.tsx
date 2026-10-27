import { useId, useState } from 'react';
import { toast } from 'sonner';
import type { AutomationRunView } from '@/api/generated/api.types';
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

export function AutomationRunRecovery({ run }: { run: AutomationRunView }) {
  const profile = useRead('getCurrentProfile');
  const resolve = useMutate('resolveAutomationRunCleanup');
  const id = useId();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  async function submit() {
    if (!run.attemptId) return;
    try {
      await resolve.mutateAsync({
        id: run.actionId,
        runId: run.id,
        data: { attemptId: run.attemptId, reason: reason.trim(), acknowledged },
      });
      setOpen(false);
      toast.success('Retained capacity released. The run result is unchanged.');
    } catch {
      // Keep the operator's input and display the request error below.
    }
  }
  if (!run.cleanupPending || !run.finishedAt) return null;
  const administrator = profile.data?.data.authorization.isAdministrator === true;
  return (
    <>
      <AlertMessage
        type="warning"
        title="Cleanup pending"
        action={
          administrator && (
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
          )
        }>
        This attempt still holds execution capacity while Citadel verifies that the script stopped.
      </AlertMessage>
      <Dialog
        open={open}
        onOpenChange={(value) => {
          if (!resolve.isPending) setOpen(value);
        }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Resolve automation cleanup</DialogTitle>
            <DialogDescription>
              Check the original host and stop any remaining script for this attempt. Once other operations finish,
              restart the original Agent. Citadel verifies that its identity changed and it still uses the same Docker
              host.
            </DialogDescription>
          </DialogHeader>
          <p className="break-all font-mono text-xs">Attempt: {run.attemptId}</p>
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
              I verified that this attempt’s script has stopped and restarted the original Agent.
            </Label>
          </div>
          <p className="text-sm text-muted-foreground">
            Releases this reservation and records your acknowledgment in Activities. The run result is unchanged.
          </p>
          {resolve.isError && (
            <AlertMessage type="error">
              {resolve.validationErrors ?? 'Could not resolve cleanup. Refresh the run and try again.'}
            </AlertMessage>
          )}
          <DialogFooter>
            <Button variant="outline" disabled={resolve.isPending} onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button disabled={!acknowledged || !reason.trim() || resolve.isPending} onClick={() => void submit()}>
              Release capacity
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
