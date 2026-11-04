import { useState } from 'react';
import { useSetAtom } from 'jotai';
import { useQueryClient } from '@tanstack/react-query';
import { ArrowRight, CloudDownload, RefreshCw } from 'lucide-react';
import { toast } from 'sonner';
import type { AgentReviewView, AgentUpdatesView, UpdateReviewView, UpdateStatusView } from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';
import { randomUuid } from '@/lib/utils';
import { pendingInstanceUpdateAtom } from '@/lib/instance-update-state';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { AlertMessage } from '@/components/custom/alert-message';
import { CoreReviewDetails } from './installation';
import { AgentUpdateProgress } from './agent-progress';

type Component = 'Core' | 'Agents';

export function ReviewUpdates({
  status,
  agents,
  selected = [],
  allowed = true,
  busy = false,
  agentsFailed = false,
  onAgentsApproved,
}: {
  status?: UpdateStatusView;
  agents?: AgentUpdatesView;
  selected?: string[];
  allowed?: boolean;
  busy?: boolean;
  agentsFailed?: boolean;
  onAgentsApproved?: () => void;
}) {
  const queryClient = useQueryClient();
  const remember = useSetAtom(pendingInstanceUpdateAtom);
  const [open, setOpen] = useState(false);
  const [component, setComponent] = useState<Component>();
  const [coreReview, setCoreReview] = useState<UpdateReviewView>();
  const [agentReview, setAgentReview] = useState<AgentReviewView>();
  const [requestId, setRequestId] = useState('');
  const [operationId, setOperationId] = useState<string>();
  const prepareCore = useMutate('reviewInstanceUpdate', {
    onSuccess: ({ data }) => {
      setCoreReview(data);
      setRequestId(randomUuid());
    },
  });
  const prepareAgents = useMutate('reviewAgentUpdates', {
    onSuccess: ({ data }) => {
      setAgentReview(data);
      setRequestId(randomUuid());
    },
  });
  const approveCore = useMutate('approveInstanceUpdate', {
    onSuccess: async ({ data }) => {
      if (!status) return;
      try {
        remember({ instanceId: status.instanceId, id: data.id, restartUntil: Date.now() + 15 * 60_000 });
      } catch {
        // Browser storage cannot undo a server-approved operation.
      }
      await queryClient.cancelQueries({ queryKey: ['getInstanceUpdates'] });
      queryClient.setQueryData(['getInstanceUpdates', {}], { data: { ...status, operation: data } });
      setOpen(false);
      toast.success('Update approved', { description: 'Citadel will prepare a recovery checkpoint and restart.' });
    },
  });
  const approveAgents = useMutate('approveAgentUpdates', {
    onSuccess: async ({ data }) => {
      await queryClient.cancelQueries({ queryKey: ['getAgentUpdates'] });
      queryClient.setQueryData<{ data: AgentUpdatesView }>(['getAgentUpdates', {}], (previous) => {
        const state = previous?.data ?? agents;
        if (!state) return previous;
        const current = state.operation;
        const operation =
          current?.id === data.id && Date.parse(current.updatedAt) > Date.parse(data.updatedAt) ? current : data;
        return { ...previous, data: { ...state, operation } };
      });
      queryClient.setQueryData(
        ['getAgentUpdateOperation', { id: data.id }],
        (previous: unknown) => previous ?? { data },
      );
      setOperationId(data.id);
      setAgentReview(undefined);
      onAgentsApproved?.();
      toast.success('Agent updates started');
    },
  });
  const targets =
    agents?.targets.filter(
      (target) => target.status === 'Available' && (!selected.length || selected.includes(target.id)),
    ) ?? [];
  const coreEligible = !!status?.available && !status.installationUnavailableReason && status.checksAllowed;
  const agentsEligible = !agentsFailed && targets.length > 0;
  const preparing = prepareCore.isPending || prepareAgents.isPending;
  const approving = approveCore.isPending || approveAgents.isPending;
  const blocked =
    busy ||
    !!status?.operation ||
    (!!agents?.operation && !agents.operation.finished) ||
    (!!agents?.scheduleRun && agents.scheduleRun.phase !== 'Finished');
  const canReview = allowed && !blocked && (coreEligible || agentsEligible);
  const review = component === 'Core' ? coreReview : agentReview;
  const prepareFailed = component === 'Core' ? prepareCore.isError : prepareAgents.isError;
  const approveFailed = component === 'Core' ? approveCore.isError : approveAgents.isError;
  const prepare = (scope: Component) => {
    setComponent(scope);
    if (scope === 'Core' && coreEligible) prepareCore.mutate({ data: { version: status!.available!.version } });
    if (scope === 'Agents' && agentsEligible)
      prepareAgents.mutate({ data: { targets: targets.map((target) => target.id) } });
  };
  return (
    <>
      <div className="flex flex-wrap items-center justify-end gap-2">
        {agents?.operation && (
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setOperationId(agents.operation!.id);
              setOpen(true);
            }}>
            {agents.operation.finished ? 'View last update' : 'View progress'}
          </Button>
        )}
        <Button
          type="button"
          disabled={!canReview || preparing || approving}
          onClick={() => {
            setCoreReview(undefined);
            setAgentReview(undefined);
            setOperationId(undefined);
            setComponent(undefined);
            prepareCore.reset();
            prepareAgents.reset();
            approveCore.reset();
            approveAgents.reset();
            setOpen(true);
            if (!(coreEligible && agentsEligible)) prepare(coreEligible ? 'Core' : 'Agents');
          }}>
          <CloudDownload className="size-4" />
          Review and update
        </Button>
      </div>
      <Dialog
        open={open}
        onOpenChange={(value) => {
          if (!approving) setOpen(value);
        }}>
        <DialogContent className="max-h-[calc(100dvh_-_2rem)] w-[calc(100vw_-_2rem)] gap-5 overflow-y-auto">
          {operationId ? (
            <>
              <AgentUpdateProgress payload={{ id: operationId }} />
              <DialogFooter>
                <Button variant="outline" onClick={() => setOpen(false)}>
                  Close
                </Button>
              </DialogFooter>
            </>
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>
                  {component === 'Core'
                    ? 'Update Citadel'
                    : component === 'Agents'
                      ? 'Update Agents'
                      : 'Review updates'}
                </DialogTitle>
                <DialogDescription>
                  {component === 'Core'
                    ? 'Install the new version on this Citadel instance.'
                    : component === 'Agents'
                      ? 'Agents update one at a time. Core keeps running. Closing the dialog does not cancel an approved update.'
                      : 'Choose which component to update. Update Core first to make its matching Agent release available.'}
                </DialogDescription>
              </DialogHeader>
              {!component && (
                <div className="grid gap-3">
                  <Button variant="outline" disabled={!canReview || !coreEligible} onClick={() => prepare('Core')}>
                    Citadel Core · {status?.available?.version}
                  </Button>
                  <Button variant="outline" disabled={!canReview || !agentsEligible} onClick={() => prepare('Agents')}>
                    {targets.length} Agent update{targets.length === 1 ? '' : 's'}
                  </Button>
                </div>
              )}
              {preparing && (
                <p role="status" className="flex items-center gap-2 text-sm text-muted-foreground">
                  <RefreshCw className="size-4 animate-spin" />
                  Verifying update eligibility…
                </p>
              )}
              {component === 'Core' && coreReview && <CoreReviewDetails review={coreReview} />}
              {component === 'Agents' && agentReview && (
                <>
                  <div className="divide-y rounded-md border">
                    {agentReview.targets.map((target) => (
                      <div key={target.id} className="space-y-2 p-3 text-sm">
                        <p className="font-medium">
                          {target.name}
                          {target.nodes > 1 ? ` · ${target.nodes} nodes` : ''}
                        </p>
                        <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3 text-center">
                          <span className="break-all">{target.installedVersion}</span>
                          <ArrowRight className="size-4 text-muted-foreground" />
                          <span className="break-all">{target.availableVersion}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Active work must finish before replacement. An installation failure stops the remaining updates.
                  </p>
                </>
              )}
              {prepareFailed && (
                <AlertMessage type="error" title="Review could not be prepared">
                  Check the current status and try again.
                </AlertMessage>
              )}
              {approveFailed && (
                <div role="alert">
                  <AlertMessage type="error" title="Approval could not be confirmed">
                    Try again. Retrying uses the same request and will not start a second update.
                  </AlertMessage>
                </div>
              )}
              <DialogFooter className="border-t pt-4">
                <Button variant="outline" disabled={approving} onClick={() => setOpen(false)}>
                  Cancel
                </Button>
                {component && prepareFailed && !review && (
                  <Button disabled={!canReview || preparing} onClick={() => prepare(component)}>
                    Retry review
                  </Button>
                )}
                {review && (
                  <Button
                    disabled={approving || blocked || !allowed}
                    onClick={() => {
                      const data = { reviewId: review.id, requestId };
                      if (component === 'Core') approveCore.mutate({ data });
                      else approveAgents.mutate({ data });
                    }}>
                    {approving ? 'Approving…' : component === 'Core' ? 'Update and restart' : 'Update now'}
                  </Button>
                )}
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
