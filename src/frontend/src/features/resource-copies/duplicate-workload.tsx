import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router';
import { ArrowLeft, ArrowRight, Check, Copy, Database, Loader2, ShieldCheck, Trash2 } from 'lucide-react';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { ResourceSelectorField } from '@/components/custom/common';
import { DuplicateMenu } from '@/components/custom/duplicate-menu';
import { StepIndicator } from '@/components/custom/step-indicator';
import { AlertMessage } from '@/components/custom/alert-message';
import { StateBadge } from '@/components/custom/state-badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead } from '@/lib/hooks';
import { useResourceCopyGroup } from './useResourceCopyGroup';
import { requestErrorMessage } from '@/lib/request-error';
import { ResourceCopyDataMode } from '@/api/generated/api.types';
import type {
  ResourceCopyKind,
  ResourceCopyEligibility,
  ResourceCopyPreview,
  ResourceCopySelection,
  ResourceCopyProgress,
} from '@/api/generated/api.types';

const steps = [
  { id: 'configure', title: 'Configure', description: 'Name your copy' },
  { id: 'data', title: 'Select data', description: 'Choose how to copy data' },
  { id: 'review', title: 'Review', description: 'Confirm and create' },
] as const;
const finished = (status?: string) => ['Succeeded', 'Failed', 'Cancelled', 'Cleaned'].includes(status ?? '');

export function DuplicateWorkload({
  kind,
  sourceId,
  sourceName,
  configurationHref,
  copyEligibility,
  disabled,
}: {
  kind: ResourceCopyKind;
  sourceId: string;
  sourceName: string;
  configurationHref: string;
  copyEligibility?: ResourceCopyEligibility | null;
  disabled?: boolean;
}) {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const operationId = params.get('copyOperation') ?? '';
  const suggestedName = `${sourceName.replace(/[^A-Za-z0-9._-]/g, '-').slice(0, 59)}-copy`;
  const [open, setOpen] = useState(Boolean(operationId));
  const [step, setStep] = useState('configure');
  const [dataMode, setDataMode] = useState(ResourceCopyDataMode.CurrentData);
  const [consistencyAcknowledged, setConsistencyAcknowledged] = useState(false);
  const currentData = dataMode === ResourceCopyDataMode.CurrentData;
  const [name, setName] = useState(suggestedName);
  const [destinationPlatform, setDestinationPlatform] = useState<string>();
  const [destinationNode, setDestinationNode] = useState<string>();
  const targets = useRead('getResourceCopyTargets', { query: { kind, sourceId } }, { enabled: open && !operationId });
  const targetCatalog = targets.data?.data;
  const platformId = destinationPlatform ?? targetCatalog?.sourcePlatformId;
  const crossPlatform = Boolean(platformId && targetCatalog && platformId !== targetCatalog.sourcePlatformId);
  const nodeId = destinationNode ?? (!crossPlatform ? (targetCatalog?.sourceNodeId ?? undefined) : undefined);
  const nodes = useRead(
    'listSwarmNodes',
    { platformId: platformId ?? '' },
    {
      enabled: open && !operationId && Boolean(platformId) && Boolean(targetCatalog?.swarm),
    },
  );
  const availableNodes = (nodes.data?.data.items ?? [])
    .filter(
      (node) => !node.isStale && node.status.toLowerCase() === 'ready' && node.availability.toLowerCase() === 'active',
    )
    .map((node) => ({ id: node.id, name: node.hostname || node.id }));
  const destinationReady = Boolean(platformId && targetCatalog && !targets.isError && (!targetCatalog.swarm || nodeId));

  const [preview, setPreview] = useState<ResourceCopyPreview>();
  const [selections, setSelections] = useState<Record<string, ResourceCopySelection>>({});
  const [acknowledged, setAcknowledged] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const previewCopy = useMutate('previewResourceCopy');
  const startCopy = useMutate('startResourceCopy');
  const cancelCopy = useMutate('cancelResourceCopy');
  const cleanupCopy = useMutate('cleanupResourceCopy');
  const operation = useResourceCopyGroup(operationId, open);
  const busy = previewCopy.isPending || startCopy.isPending || cancelCopy.isPending || cleanupCopy.isPending;
  const error =
    previewCopy.error ||
    startCopy.error ||
    cancelCopy.error ||
    cleanupCopy.error ||
    (operationId ? operation.error : targets.error || nodes.error);
  const progress = operation.data?.data;
  const terminal = finished(progress?.status);

  useEffect(() => {
    if (open) heading.current?.focus();
  }, [step, open]);

  async function prepare(next: 'data' | 'review') {
    try {
      const { data: value } = await previewCopy.mutateAsync({
        data: {
          kind,
          sourceId,
          ...(destinationPlatform ? { destinationPlatformId: destinationPlatform } : {}),
          ...(destinationNode ? { destinationNodeId: destinationNode } : {}),
          name: name.trim(),
          dataMode,
          consistencyAcknowledged: currentData && consistencyAcknowledged,
          selections: !currentData && next === 'review' ? Object.values(selections) : [],
        },
      });
      setPreview(value);
      setAcknowledged(false);
      setStep(next === 'review' && value.blockers.length ? 'data' : next);
    } catch {
      /* Keep the current step and show the request error. */
    }
  }
  async function start() {
    const id = operationId || preview?.id;
    if (!id) return;
    // Persist the operation link before submitting. A lost response or refresh
    // can only retry the same durable review, never create another destination.
    setParams(
      (current) => {
        current.set('copyOperation', id);
        return current;
      },
      { replace: true },
    );
    try {
      await startCopy.mutateAsync({ id });
    } catch {
      /* The operation link remains available for reconciliation/retry. */
    }
  }
  async function cancel() {
    try {
      await cancelCopy.mutateAsync({ id: operationId });
    } catch {
      /* Show error. */
    }
  }
  function reset() {
    setParams(
      (current) => {
        current.delete('copyOperation');
        return current;
      },
      { replace: true },
    );
    setName(suggestedName);
    setDestinationPlatform(undefined);
    setDestinationNode(undefined);
    setPreview(undefined);
    setSelections({});
    setStep('configure');
    setConsistencyAcknowledged(false);
    setAcknowledged(false);
    previewCopy.reset();
    startCopy.reset();
    cancelCopy.reset();
    cleanupCopy.reset();
  }
  const canSelect =
    preview?.volumes.length &&
    (currentData ? consistencyAcknowledged : preview.volumes.every((volume) => selections[volume.source]));
  const selectedCount = Object.keys(selections).length;
  const noCopyableVolumes = step === 'data' && preview?.volumes.length === 0;
  const dataDisabledReason =
    copyEligibility?.status === 'Available'
      ? undefined
      : (copyEligibility?.reason ?? 'Copy availability could not be checked. Refresh the resource to try again.');

  return (
    <>
      <DuplicateMenu
        disabled={disabled}
        dataDisabledReason={operationId ? undefined : dataDisabledReason}
        onSelect={(mode) => {
          if (mode === 'configuration') navigate(configurationHref);
          else setOpen(true);
        }}
      />
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="flex max-h-[90dvh] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl">
          <DialogHeader className="border-b px-6 py-5 pr-12">
            <DialogTitle className="flex items-center gap-2">
              <Copy className="size-4 text-primary" />
              Duplicate with data
            </DialogTitle>
            <DialogDescription>
              Copy {sourceName} into independent volumes using current data or existing backups.
            </DialogDescription>
          </DialogHeader>
          {!operationId && (
            <div className="border-b bg-muted/20 px-6 py-5">
              <StepIndicator
                steps={steps}
                currentStep={step}
                disabled={busy}
                onStepChange={(id) => {
                  setStep(id);
                  setAcknowledged(false);
                }}
              />
            </div>
          )}
          <div className="min-h-64 space-y-5 overflow-y-auto px-6 py-6 text-sm">
            {error && (
              <AlertMessage type="error" title="Copy could not continue">
                {requestErrorMessage(error)}
              </AlertMessage>
            )}
            {operationId ? (
              <>
                {progress ? (
                  <CopyProgressCard progress={progress} />
                ) : (
                  <p role="status" className="flex items-center gap-2 text-muted-foreground">
                    <Loader2 className="size-4 animate-spin" />
                    Loading copy progress…
                  </p>
                )}
                {!terminal && (
                  <p className="text-xs text-muted-foreground">
                    You can close this window. Once started, the copy continues in the background; this page URL
                    preserves its progress link.
                  </p>
                )}
              </>
            ) : (
              <>
                <h3 ref={heading} tabIndex={-1} className="font-medium outline-none">
                  {step === 'configure'
                    ? 'Give the copy a name'
                    : step === 'data'
                      ? noCopyableVolumes
                        ? 'Data copying is unavailable'
                        : currentData
                          ? 'Copy the current volume data'
                          : 'Choose a backup for each volume'
                      : 'Ready to create your copy'}
                </h3>
                {step === 'configure' && (
                  <div className="space-y-4">
                    <div className="space-y-2">
                      <Label htmlFor={`copy-name-${sourceId}`}>Name</Label>
                      <Input
                        id={`copy-name-${sourceId}`}
                        value={name}
                        maxLength={64}
                        disabled={busy}
                        onChange={(event) => {
                          setName(event.target.value);
                          setPreview(undefined);
                          setSelections({});
                        }}
                        autoComplete="off"
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor={`copy-platform-${sourceId}`}>Destination platform</Label>
                      <ResourceSelectorField
                        className='max-w-full w-full'
                        id={`copy-platform-${sourceId}`}
                        items={targetCatalog?.platforms ?? []}
                        selected={platformId}
                        allowClear={false}
                        disabled={busy || targets.isLoading}
                        placeholder={targets.isLoading ? 'Loading platforms…' : 'Select a platform'}
                        searchPlaceholder="Search compatible platforms…"
                        onSelect={(platform) => {
                          setDestinationPlatform(platform?.id);
                          setDestinationNode(undefined);
                          setPreview(undefined);
                          setSelections({});
                          setAcknowledged(false);
                          if (platform?.id !== targetCatalog?.sourcePlatformId) {
                            setDataMode(ResourceCopyDataMode.ExistingBackup);
                            setConsistencyAcknowledged(false);
                          }
                        }}
                      />
                      <p className="text-xs text-muted-foreground">
                        Only {targetCatalog?.swarm ? 'Swarm' : 'Standalone'} platforms you can write to are listed.
                      </p>
                    </div>
                    {targetCatalog?.swarm && (
                      <div className="space-y-2">
                        <Label htmlFor={`copy-node-${sourceId}`}>Destination node</Label>
                        <ResourceSelectorField
                          id={`copy-node-${sourceId}`}
                          items={availableNodes}
                          selected={
                            nodeId
                              ? (availableNodes.find((node) => node.id === nodeId) ?? { id: nodeId, name: nodeId })
                              : undefined
                          }
                          allowClear={false}
                          disabled={busy || nodes.isLoading || currentData}
                          placeholder={nodes.isLoading ? 'Loading nodes…' : 'Select a Ready, Active node'}
                          onSelect={(node) => {
                            setDestinationNode(node?.id);
                            setPreview(undefined);
                            setSelections({});
                          }}
                        />
                        <p className="text-xs text-muted-foreground">
                          Restored volumes and the copied service will be pinned to this node.
                        </p>
                      </div>
                    )}
                    <div className="space-y-2">
                      <Label htmlFor={`copy-data-mode-${sourceId}`}>Data source</Label>
                      <Select
                        value={dataMode}
                        disabled={busy}
                        onValueChange={(value) => {
                          setDataMode(value as ResourceCopyDataMode);
                          setPreview(undefined);
                          setSelections({});
                          setConsistencyAcknowledged(false);
                        }}>
                        <SelectTrigger id={`copy-data-mode-${sourceId}`} className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            value={ResourceCopyDataMode.CurrentData}
                            disabled={
                              crossPlatform || (Boolean(destinationNode) && nodeId !== targetCatalog?.sourceNodeId)
                            }>
                            Copy current data
                          </SelectItem>
                          <SelectItem value={ResourceCopyDataMode.ExistingBackup}>Use an existing backup</SelectItem>
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        {currentData
                          ? 'Take a temporary snapshot and restore it into new volumes. No backup policy or repository is required.'
                          : crossPlatform
                            ? 'Cross-platform copies use existing backups in an S3-compatible repository reachable from the destination.'
                            : 'Restore an available backup for each volume. You need permission to restore the selected backups.'}
                      </p>
                    </div>
                    <p className="text-muted-foreground">
                      Standalone platforms copy to Standalone; Swarm platforms copy to Swarm. Current data can only be
                      copied on its original platform and node. You can edit the copy before its first Apply.
                    </p>
                    <div className="flex gap-3 rounded-md border bg-muted/20 p-4">
                      <ShieldCheck className="mt-0.5 size-4 shrink-0 text-primary" />
                      <p>Data restores into new volumes. The copy stays undeployed until you choose Apply.</p>
                    </div>
                  </div>
                )}
                {step === 'data' && preview && (
                  <div className="space-y-4">
                    <p className="text-xs text-muted-foreground">
                      Platform: {preview.platformName}
                      {!currentData &&
                        !noCopyableVolumes &&
                        ` · ${selectedCount} of ${preview.volumes.length} volumes selected`}
                    </p>
                    {preview.blockers
                      .filter((message) => !message.startsWith('Select an available backup for '))
                      .map((message) => (
                        <AlertMessage key={message} type="warning">
                          {message}
                        </AlertMessage>
                      ))}
                    {noCopyableVolumes && (
                      <p className="text-muted-foreground">
                        You can still copy the configuration without restoring data. In-memory data and container
                        writable layers are not included.
                      </p>
                    )}
                    {preview.volumes.map((volume, index) => (
                      <div key={volume.source} className="space-y-3 rounded-md border p-4">
                        <Label htmlFor={`copy-snapshot-${index}`} className="flex min-w-0 items-center gap-2">
                          <Database className="size-4 shrink-0 text-muted-foreground" />
                          <span className="truncate" title={volume.source}>
                            {volume.source}
                          </span>
                        </Label>
                        {volume.dockerNodeId && (
                          <p className="break-all text-xs text-muted-foreground">Node: {volume.dockerNodeId}</p>
                        )}
                        {currentData ? (
                          <p className="text-xs text-muted-foreground">
                            A temporary snapshot will be taken when this copy starts.
                          </p>
                        ) : (
                          <>
                            <Select
                              disabled={busy || volume.snapshots.length === 0}
                              value={selections[volume.source]?.backupRunItemId ?? ''}
                              onValueChange={(itemId) => {
                                const snapshot = volume.snapshots.find((item) => item.backupRunItemId === itemId);
                                if (snapshot)
                                  setSelections((current) => ({
                                    ...current,
                                    [volume.source]: {
                                      volume: volume.source,
                                      backupRunId: snapshot.backupRunId,
                                      backupRunItemId: snapshot.backupRunItemId,
                                    },
                                  }));
                              }}>
                              <SelectTrigger id={`copy-snapshot-${index}`} className="w-full">
                                <SelectValue
                                  placeholder={volume.snapshots.length ? 'Select a backup' : 'No available backups'}
                                />
                              </SelectTrigger>
                              <SelectContent>
                                {volume.snapshots.map((snapshot) => (
                                  <SelectItem key={snapshot.backupRunItemId} value={snapshot.backupRunItemId}>
                                    {snapshot.policyName} ·{' '}
                                    {snapshot.completedAt
                                      ? new Date(snapshot.completedAt).toLocaleString()
                                      : snapshot.backupRunId.slice(0, 8)}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                            {!volume.snapshots.length && (
                              <p className="text-xs text-muted-foreground">
                                {crossPlatform
                                  ? 'No restorable S3 backup is available for this volume. Create one and ensure the destination can access its repository.'
                                  : 'No restorable backup is available. Go back and choose Copy current data, or create a backup first.'}
                              </p>
                            )}
                          </>
                        )}
                      </div>
                    ))}
                    {currentData && !noCopyableVolumes && (
                      <label
                        htmlFor={`copy-consistency-${sourceId}`}
                        className="flex items-start gap-3 rounded-md border p-4">
                        <Checkbox
                          id={`copy-consistency-${sourceId}`}
                          disabled={busy}
                          checked={consistencyAcknowledged}
                          onCheckedChange={(value) => setConsistencyAcknowledged(value === true)}
                        />
                        <span className="text-sm">
                          The source data is ready for a file-level snapshot. I have stopped or quiesced applications
                          that require it. Citadel will not stop or restart the source automatically.
                        </span>
                      </label>
                    )}
                  </div>
                )}
                {step === 'review' && preview && (
                  <div className="space-y-4">
                    <dl className="grid grid-cols-2 gap-4 rounded-md border bg-muted/20 p-4">
                      <div>
                        <dt className="text-xs text-muted-foreground">New resource</dt>
                        <dd className="mt-1 break-all font-medium">{preview.name}</dd>
                      </div>
                      <div>
                        <dt className="text-xs text-muted-foreground">Destination platform</dt>
                        <dd className="mt-1 font-medium">{preview.platformName}</dd>
                        {preview.destinationNodeId && (
                          <dd className="mt-1 break-all text-xs text-muted-foreground">
                            Node:{' '}
                            {availableNodes.find((node) => node.id === preview.destinationNodeId)?.name ??
                              preview.destinationNodeId}
                          </dd>
                        )}
                      </div>
                    </dl>
                    <p className="text-sm text-muted-foreground">
                      {currentData
                        ? 'Data source: current volumes (temporary snapshots)'
                        : 'Data source: selected backups'}
                    </p>
                    <div className="space-y-3">
                      {preview.volumes.map((volume) => (
                        <div key={volume.source} className="flex min-w-0 gap-3">
                          <Database className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                          <div className="min-w-0 space-y-1">
                            <p className="break-all font-medium">{volume.source}</p>
                            <p className="break-all text-xs text-muted-foreground">New volume: {volume.target}</p>
                            {volume.snapshots
                              .filter(
                                (snapshot) => snapshot.backupRunItemId === selections[volume.source]?.backupRunItemId,
                              )
                              .map((snapshot) => (
                                <p key={snapshot.backupRunItemId} className="text-xs text-muted-foreground">
                                  Backup: {snapshot.policyName} ·{' '}
                                  {snapshot.completedAt
                                    ? new Date(snapshot.completedAt).toLocaleString()
                                    : snapshot.backupRunId.slice(0, 8)}
                                </p>
                              ))}
                          </div>
                        </div>
                      ))}
                    </div>
                    <ul className="space-y-2 border-t pt-4 text-xs leading-relaxed text-muted-foreground">
                      {preview.warnings.map((warning) => (
                        <li key={warning} className="flex gap-2">
                          <Check className="mt-0.5 size-3.5 shrink-0" />
                          {warning}
                        </li>
                      ))}
                    </ul>
                    <label
                      htmlFor={`copy-confirm-${sourceId}`}
                      className="flex cursor-pointer items-start gap-3 rounded-md border p-4">
                      <Checkbox
                        id={`copy-confirm-${sourceId}`}
                        checked={acknowledged}
                        disabled={busy}
                        onCheckedChange={(value) => setAcknowledged(value === true)}
                      />
                      <span className="leading-snug">
                        I’ve reviewed the data source and shared references. Create the copy without deploying it.
                      </span>
                    </label>
                  </div>
                )}
              </>
            )}
          </div>
          <DialogFooter className="shrink-0 border-t bg-muted/10 px-6 py-4">
            {operationId ? (
              <>
                <Button variant="outline" onClick={() => setOpen(false)}>
                  Close
                </Button>
                {progress?.status === 'Review' && (
                  <Button disabled={busy} onClick={start}>
                    Retry start
                  </Button>
                )}
                {progress && !terminal && (
                  <Button
                    variant="outline"
                    disabled={busy || ['Cancelling', 'Cleaning'].includes(progress.status)}
                    onClick={cancel}>
                    Cancel copy
                  </Button>
                )}
                {progress && ['Failed', 'Cancelled'].includes(progress.status) && progress.restores.length > 0 && (
                  <ActionWithDialog
                    name={progress.name}
                    title="Clean up volumes"
                    icon={<Trash2 className="size-4" />}
                    variant="outline"
                    disabled={busy}
                    description="Remove unused destination volumes created by this copy. The source volumes are preserved. Ownership and attached containers are checked before removal."
                    onClick={async () => {
                      await cleanupCopy.mutateAsync({ id: operationId });
                    }}
                  />
                )}
                {((terminal && !progress?.destinationId) || operation.isError) && (
                  <Button variant="outline" onClick={reset}>
                    New copy
                  </Button>
                )}
                {progress?.destinationId && (
                  <Button asChild>
                    <Link
                      to={destinationHref(progress)}
                      onClick={() => {
                        reset();
                        setOpen(false);
                      }}>
                      Open {kind === 'SwarmService' ? 'service' : kind.toLowerCase()}
                      <ArrowRight className="size-4" />
                    </Link>
                  </Button>
                )}
              </>
            ) : (
              <>
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    step === 'configure' ? setOpen(false) : setStep(step === 'review' ? 'data' : 'configure')
                  }>
                  <ArrowLeft className="size-4" />
                  {step === 'configure' ? 'Cancel' : 'Back'}
                </Button>
                {noCopyableVolumes ? (
                  <Button
                    disabled={busy}
                    onClick={() => {
                      setOpen(false);
                      navigate(configurationHref);
                    }}>
                    <Copy className="size-4" />
                    Configuration only
                  </Button>
                ) : (
                  <Button
                    disabled={
                      busy ||
                      (step === 'configure'
                        ? !name.trim() || !destinationReady
                        : step === 'data'
                          ? !canSelect
                          : !acknowledged)
                    }
                    onClick={() =>
                      step === 'configure' ? prepare('data') : step === 'data' ? prepare('review') : start()
                    }>
                    {busy && <Loader2 className="size-4 animate-spin" />}
                    {step === 'review'
                      ? 'Create copy'
                      : step === 'data'
                        ? 'Review copy'
                        : currentData
                          ? 'Review volumes'
                          : 'Choose backups'}
                    {!busy && <ArrowRight className="size-4" />}
                  </Button>
                )}
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
function destinationHref(progress: ResourceCopyProgress) {
  if (progress.kind === 'Deployment') return `/deployments/edit/${progress.destinationId}/`;
  if (progress.kind === 'Stack') return `/stacks/edit/${progress.destinationId}/`;
  return `/swarm-services/edit/${progress.destinationId}/`;
}
function CopyProgressCard({ progress }: { progress: ResourceCopyProgress }) {
  return (
    <div className="space-y-4" aria-live="polite">
      <div className="flex items-center justify-between gap-4">
        <h3 className="break-all font-medium">{progress.name}</h3>
        <CopyStatusBadge status={progress.status} />
      </div>
      {progress.message && <p className="text-muted-foreground">{progress.message}</p>}
      {progress.restores.map((restore) => (
        <div key={restore.id} className="space-y-2 rounded-md border p-4">
          <div className="flex items-start justify-between gap-3">
            <span className="break-all font-medium">{restore.source}</span>
            <CopyStatusBadge status={restore.status} />
          </div>
          <p className="break-all text-xs text-muted-foreground">Destination: {restore.target}</p>
          {restore.message && <p className="text-xs text-muted-foreground">{restore.message}</p>}
        </div>
      ))}
    </div>
  );
}

function CopyStatusBadge({ status }: { status: string }) {
  const active = ['Queued', 'Preparing', 'Running', 'Restoring', 'Cancelling', 'Cleaning'].includes(status);
  return (
    <StateBadge
      value={status}
      kind="run"
      indicator={active ? <Loader2 aria-hidden="true" className="animate-spin" /> : undefined}
    />
  );
}
