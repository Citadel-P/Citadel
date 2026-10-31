import { ScriptRunStatus, ScriptRunTrigger, ScriptRunView, AuthorizedScript } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { LastRunCell } from '@/components/custom/last-run-cell';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useMutate, useRead } from '@/lib/hooks';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { Ban, Calendar, Clock, SquareTerminal, User, type LucideIcon } from 'lucide-react';
import { useCallback, useMemo, useState, type ReactNode } from 'react';
import { toast } from 'sonner';
import { ScriptRunRecovery } from './run-recovery';
import { useScriptRuns } from '../hooks/useScriptRuns';
import { LogViewer } from '@/components/custom/log-viewer';

export function ScriptRunsTab({ resource }: { resource: AuthorizedScript }) {
  const formatDateTime = useProfileDateTimeFormatter();
  const [logRunId, setLogRunId] = useState<string | undefined>();
  const {
    mutateAsync: cancelRun,
    isPending: cancelPending,
    validationErrors: cancelErrors,
  } = useMutate('cancelScriptRun');
  const { data, isLoading } = useScriptRuns(resource.id);

  const runs = useMemo(() => data?.data.runs ?? [], [data?.data.runs]);
  const selectedRun = useMemo(() => runs.find((run) => run.id === logRunId), [logRunId, runs]);

  const logs = useRead('getScriptRunLogs', { id: resource.id, runId: logRunId ?? '' }, { enabled: Boolean(logRunId) });

  const handleCancel = useCallback(
    async (run: ScriptRunView) => {
      try {
        await cancelRun({ id: resource.id, runId: run.id });
        toast.success('Run cancellation requested');
      } catch {
        toast.error(cancelErrors ?? 'Failed to cancel run');
      }
    },
    [cancelRun, cancelErrors, resource.id],
  );

  const columns = useMemo(
    () => runColumns(setLogRunId, handleCancel, cancelPending, formatDateTime),
    [cancelPending, handleCancel, formatDateTime],
  );

  return (
    <div className="flex flex-col gap-4">
      <DataTable columns={columns} data={runs} isLoading={isLoading} />

      <ScriptRunLogsSheet
        open={Boolean(logRunId)}
        run={selectedRun}
        formatDateTime={formatDateTime}
        logs={logs.data?.data.logs ?? ''}
        isLoading={logs.isLoading}
        onOpenChange={(open) => {
          if (!open) setLogRunId(undefined);
        }}
      />
    </div>
  );
}

const runColumns = (
  onSelectLog: (id: string) => void,
  onCancel: (run: ScriptRunView) => void,
  cancelPending: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<StockFeatures, ScriptRunView>[] => [
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Run" column={column} />,
    cell: ({ row }) => (
      <div className="flex flex-col items-start gap-1">
        <Button
          type="button"
          variant="ghost"
          className="h-auto p-0 hover:bg-transparent hover:underline"
          aria-label={`View ${statusLabel(row.original.status).toLowerCase()} run details`}
          onClick={() => onSelectLog(row.original.id)}>
          <LastRunCell status={row.original.status} labelPrefix="Run">
            {statusLabel(row.original.status)}
          </LastRunCell>
        </Button>
        <span className="pl-5.5 text-xs text-muted-foreground">{row.original.trigger} run</span>
        {row.original.cleanupPending && !isActiveRun(row.original) && (
          <span className="pl-5.5 text-xs text-muted-foreground">Cleanup pending</span>
        )}
      </div>
    ),
    sortFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'durationMs',
    header: ({ column }) => <SortableCell cellName="Result" column={column} />,
    cell: ({ row }) => (
      <div className="flex flex-col gap-1">
        <span className="inline-flex items-center gap-1.5 text-sm">
          <Clock className="size-3.5 text-muted-foreground" aria-hidden="true" />
          {runDuration(row.original)}
        </span>
        {row.original.exitCode !== null && (
          <span className="text-xs text-muted-foreground">Exit code {row.original.exitCode}</span>
        )}
      </div>
    ),
    sortFn: (rowA, rowB) => Number(rowA.original.durationMs ?? 0) - Number(rowB.original.durationMs ?? 0),
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      const cancellable = isActiveRun(row.original);
      return (
        <div className="flex justify-end gap-1">
          {cancellable && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={cancelPending}
              onClick={() => onCancel(row.original)}
              aria-label="Cancel run"
              title="Cancel run">
              <Ban className="size-3.5" />
            </Button>
          )}
        </div>
      );
    },
  },
];

function formatDuration(value: ScriptRunView['durationMs']) {
  if (value === null || value === undefined) return '-';

  const ms = Number(value);
  if (!Number.isFinite(ms)) return '-';
  if (ms < 1000) return `${ms} ms`;

  return `${(ms / 1000).toFixed(1)} s`;
}

function isActiveRun(run: Pick<ScriptRunView, 'status'>) {
  return run.status === ScriptRunStatus.Queued || run.status === ScriptRunStatus.Running;
}

function statusLabel(status: ScriptRunStatus) {
  return status === ScriptRunStatus.TimedOut ? 'Timed out' : status;
}

function runDuration(run: ScriptRunView) {
  if (run.durationMs != null) return formatDuration(run.durationMs);
  if (run.status === ScriptRunStatus.Running) return 'In progress';
  return run.startedAt ? 'Unavailable' : 'Not started';
}

function ScriptRunLogsSheet({
  open,
  run,
  formatDateTime,
  logs,
  isLoading,
  onOpenChange,
}: {
  open: boolean;
  run?: ScriptRunView;
  formatDateTime: DateTimeFormatter;
  logs: string;
  isLoading: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const profile = useRead('getCurrentProfile', undefined, { enabled: open });
  const canReadActors = profile.data?.data.authorization.isAdministrator === true;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onOpenAutoFocus={(event) => event.preventDefault()}
        side="top"
        className="mx-auto w-300 max-w-[100vw] max-h-[calc(100dvh-1rem)] overflow-y-auto rounded-b-md">
        <div className="p-2 min-w-0">
          <SheetHeader className="pr-10">
            <SheetTitle>{run ? `${run.scriptName} logs` : 'Run logs'}</SheetTitle>
            <SheetDescription>{run ? `${run.trigger} run · ${statusLabel(run.status)}` : 'Run logs'}</SheetDescription>
          </SheetHeader>

          <div className="flex flex-col gap-4 p-4 pt-0 pb-2">
            {run && (
              <dl className="flex flex-wrap gap-x-6 gap-y-2 text-xs">
                <RunDetail icon={User} label="Triggered by">
                  {run.triggeredByActorId ? (
                    <RunActor actorId={run.triggeredByActorId} enabled={open && canReadActors} />
                  ) : run.trigger === ScriptRunTrigger.Schedule ? (
                    'Schedule'
                  ) : run.trigger === ScriptRunTrigger.Webhook ? (
                    'Webhook'
                  ) : (
                    'Unavailable'
                  )}
                </RunDetail>
                {run.runAsActorId !== run.triggeredByActorId && (
                  <RunDetail icon={User} label="Run as">
                    <RunActor actorId={run.runAsActorId} enabled={open && canReadActors} />
                  </RunDetail>
                )}
                <RunDetail icon={Calendar} label="Queued">
                  <Timestamp value={run.queuedAt} formatDateTime={formatDateTime} />
                </RunDetail>
                <RunDetail icon={Clock} label="Duration">
                  {runDuration(run)}
                </RunDetail>
                {run.exitCode != null && run.exitCode !== 0 && (
                  <RunDetail icon={SquareTerminal} label="Exit code">
                    {run.exitCode}
                  </RunDetail>
                )}
              </dl>
            )}
            {run?.errorMessage && <AlertMessage type="error">{run.errorMessage}</AlertMessage>}
            {run && <ScriptRunRecovery run={run} />}
            <LogViewer
              logs={isLoading ? '' : logs}
              emptyMessage={isLoading ? 'Loading run logs…' : 'No logs available.'}
              autoScroll={false}
            />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function RunActor({ actorId, enabled }: { actorId: string; enabled: boolean }) {
  const query = useRead('getActor', { id: actorId }, { enabled: enabled && Boolean(actorId) });
  const actor = query.data?.data;
  return (
    <span title={actorId}>{actor ? actor.name : query.isLoading ? 'Loading…' : `Actor ${actorId.slice(0, 8)}`}</span>
  );
}

function RunDetail({ icon: Icon, label, children }: { icon: LucideIcon; label: string; children: ReactNode }) {
  return (
    <div className="flex min-w-0 max-w-full items-center gap-1.5">
      <Icon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <dt className="shrink-0 text-muted-foreground">{label}</dt>
      <dd className="min-w-0 truncate">{children}</dd>
    </div>
  );
}
