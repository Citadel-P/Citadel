import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { ArrowRight, CloudDownload, RefreshCw } from 'lucide-react';
import { AgentReviewView, AgentTargetView, AgentScheduleView, LicenseCapability } from '@/api/generated/api.types';
import { useRead, useMutate } from '@/lib/hooks';
import { randomUuid } from '@/lib/utils';
import { useTaskSheet } from '@/lib/atoms';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { MultiSelect } from '@/components/ui/multi-select';
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
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FormShell,
} from '@/components/custom/form-builder';
import { TimezoneSelectField } from '@/components/custom/timezone-select';

const statusLabels: Record<AgentTargetView['status'], string> = {
  Available: 'Update available',
  Current: 'Up to date',
  Offline: 'Offline',
  Unknown: 'Version unknown',
  ManualRequired: 'Manual update required',
  Incompatible: 'No compatible update',
};
function options(targets: AgentTargetView[], scheduled: boolean) {
  return targets.map((target) => ({
    value: target.id,
    label: `${target.name}${target.nodes > 1 ? ` · ${target.nodes} nodes` : ''} · ${target.installedVersion ?? 'Unknown'}${target.status === 'Available' ? ` → ${target.availableVersion}` : ''}`,
    disabled: scheduled ? !target.supported : target.status !== 'Available',
    disabledReason: target.reason ?? statusLabels[target.status],
    trailing: <span className="text-xs text-muted-foreground">{statusLabels[target.status]}</span>,
  }));
}

export function AgentUpdates() {
  const query = useRead('getAgentUpdates');
  const settings = useRead('getInstallationSettings').data?.data;
  const state = query.data?.data;
  const [selected, setSelected] = useState<string[]>([]);
  const [review, setReview] = useState<AgentReviewView>();
  const [request, setRequest] = useState('');
  const tasks = useTaskSheet('Updates');
  const prepare = useMutate('reviewAgentUpdates', {
    onSuccess: (response) => {
      setReview(response.data);
      setRequest(randomUuid());
    },
  });
  const approve = useMutate('approveAgentUpdates', {
    onSuccess: (response) => {
      setReview(undefined);
      setSelected([]);
      toast.success('Agent updates started');
      tasks.open({ kind: 'agentUpdates', payload: { id: response.data.id } });
    },
  });
  if (!state)
    return (
      <p role="status" className="text-sm text-muted-foreground">
        {query.isError ? 'Could not load Agent updates.' : 'Loading Agent updates…'}
      </p>
    );
  const available = state.targets.filter((t) => t.status === 'Available').length;
  const current = state.targets.filter((t) => t.status === 'Current').length;
  const blocked = state.targets.length - available - current;
  const pending = prepare.isPending || approve.isPending || (!!state.operation && !state.operation.finished);
  const eligible = selected.filter((id) => state.targets.some((t) => t.id === id && t.status === 'Available'));
  return (
    <section className="space-y-4" aria-label="Agent updates">
      <div className="space-y-3 rounded-md border bg-background p-4">
        <h2 className="text-sm font-medium">Agent updates</h2>
        <p className="text-sm text-muted-foreground" aria-live="polite">
          Agents: {available} {available === 1 ? 'update' : 'updates'} available · {current} up to date
          {blocked > 0 ? ` · ${blocked} unavailable` : ''}
        </p>
        {state.checkError && <p className="text-sm text-muted-foreground">{state.checkError}</p>}
        <MultiSelect
          aria-label="Agents to update"
          options={options(state.targets, false)}
          defaultValue={selected}
          onValueChange={setSelected}
          disabled={pending || !settings?.updateChecksAllowed}
          placeholder="Select Agents to update"
          hideSelectAll={false}
        />
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            disabled={pending || !eligible.length || !settings?.updateChecksAllowed}
            onClick={() => prepare.mutate({ data: { targets: eligible } })}>
            {prepare.isPending ? <RefreshCw className="size-4 animate-spin" /> : <CloudDownload className="size-4" />}
            {eligible.length > 1 ? 'Review selected updates' : 'Review update'}
          </Button>
          {state.operation && (
            <Button
              variant="ghost"
              onClick={() => tasks.open({ kind: 'agentUpdates', payload: { id: state.operation!.id } })}>
              {state.operation.finished ? 'View last update' : 'View progress'}
            </Button>
          )}
        </div>
        {state.targets.some((t) => t.status === 'ManualRequired') && (
          <a
            className="text-sm underline text-muted-foreground"
            href="https://docs.citadelplane.com/operations/upgrade-and-rollback/"
            target="_blank"
            rel="noreferrer">
            Manual Agent update guidance
          </a>
        )}
        {(prepare.isError || approve.isError) && (
          <AlertMessage title="Agent update could not start" type="error">
            Review the current Agent status and try again.
          </AlertMessage>
        )}
      </div>
      <AgentSchedule
        schedule={state.schedule}
        targets={state.targets}
        checksAllowed={!!settings?.updateChecksAllowed}
      />
      <Dialog
        open={!!review}
        onOpenChange={(open) => {
          if (!open && !approve.isPending) setReview(undefined);
        }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Update {review?.targets.length === 1 ? 'Agent' : 'selected Agents'}</DialogTitle>
            <DialogDescription>
              Agents update one at a time. Citadel Core keeps running. Closing the browser will not cancel the
              operation.
            </DialogDescription>
          </DialogHeader>
          <div className="divide-y rounded-md border">
            {review?.targets.map((target) => (
              <div key={target.id} className="space-y-2 p-3">
                <p className="text-sm font-medium">
                  {target.name}
                  {target.nodes > 1 ? ` · ${target.nodes} nodes` : ''}
                </p>
                <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3 text-center text-sm">
                  <span>{target.installedVersion}</span>
                  <ArrowRight className="size-4 text-muted-foreground" />
                  <span>{target.availableVersion}</span>
                </div>
              </div>
            ))}
          </div>
          <p className="text-sm text-muted-foreground">
            Active work must finish before replacement. An installation failure stops the remaining updates.
          </p>
          <DialogFooter>
            <Button variant="outline" disabled={approve.isPending} onClick={() => setReview(undefined)}>
              Cancel
            </Button>
            <Button
              disabled={approve.isPending || !review}
              onClick={() => review && approve.mutate({ data: { reviewId: review.id, requestId: request } })}>
              <CloudDownload className="size-4" />
              {approve.isPending ? 'Starting…' : 'Update now'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function AgentSchedule({
  schedule,
  targets,
  checksAllowed,
}: {
  schedule: AgentScheduleView;
  targets: AgentTargetView[];
  checksAllowed: boolean;
}) {
  const { hasCapability } = useLicenseEntitlements();
  const team = hasCapability(LicenseCapability.AutomatedOperations);
  const [update, setUpdate] = useState<Partial<AgentScheduleView>>({});
  const save = useMutate('updateAgentSchedule', { onSuccess: () => toast.success('Agent update schedule saved') });
  const enabled = update.enabled ?? schedule.enabled;
  const schema = useMemo(
    () => ({
      '': defineSection<AgentScheduleView>({
        title: '',
        items: [
          defineGroupField<AgentScheduleView>({
            id: 'agent-schedule',
            label: 'Schedule',
            description: 'Update only the selected Agents when a compatible release is available.',
            requiredLicense: team ? undefined : 'Team',
            items: [
              defineField({
                key: 'targets',
                label: 'Scheduled Agents',
                render: (value, set) => (
                  <MultiSelect
                    aria-label="Scheduled Agents"
                    options={[
                      ...options(targets, true),
                      ...schedule.targets
                        .filter((id) => !targets.some((t) => t.id === id))
                        .map((id) => ({
                          value: id,
                          label: `${id} · no longer registered`,
                          disabled: true,
                          disabledReason: 'Remove this target before saving the schedule.',
                        })),
                    ]}
                    defaultValue={value}
                    onValueChange={(targets) => set({ targets })}
                    disabled={!team || !checksAllowed || save.isPending}
                    placeholder="Select scheduled Agents"
                  />
                ),
              }),
              defineField({
                key: 'enabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    id="agent-update-schedule-enabled"
                    checked={value}
                    disabled={save.isPending || ((!team || !checksAllowed) && !schedule.enabled)}
                    onChange={(enabled) => set({ enabled })}
                  />
                ),
              }),
              defineField({
                key: 'cron',
                label: 'Cron',
                required: enabled,
                disabled: !enabled,
                render: (value, set) => (
                  <FieldInput
                    value={value}
                    placeholder="0 3 * * *"
                    disabled={save.isPending || !team || !checksAllowed || !enabled}
                    onChange={(cron) => set({ cron })}
                  />
                ),
              }),
              defineField({
                key: 'timeZone',
                label: 'Time Zone',
                required: enabled,
                disabled: !enabled,
                render: (value, set) => (
                  <TimezoneSelectField
                    value={value}
                    disabled={save.isPending || !team || !checksAllowed || !enabled}
                    onChange={(timeZone) => set({ timeZone })}
                    className="w-full min-w-0 max-w-100"
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [team, targets, checksAllowed, save.isPending, enabled, schedule.enabled, schedule.targets],
  );
  return (
    <>
      <FormShell
        schema={schema}
        original={schedule}
        update={update}
        setUpdate={setUpdate}
        pending={save.isPending}
        onSave={async () => {
          await save.mutateAsync({ data: { ...schedule, ...update } });
        }}
      />
      {save.isError && (
        <p role="alert" className="text-sm text-destructive">
          The schedule could not be saved. Check the cron expression and selected Agents.
        </p>
      )}
    </>
  );
}
