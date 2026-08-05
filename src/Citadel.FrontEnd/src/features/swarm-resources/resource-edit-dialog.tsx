import { lazy, Suspense, useState } from 'react';
import { Pencil, Save } from 'lucide-react';
import { toast } from 'sonner';
import { ProblemDetails, SwarmConfigView, SwarmSecretView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { KeyValuePairInput, KVPair } from '@/components/custom/key-value-pair-input';
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
import { hasCapability } from '@/lib/resource-capabilities';
import { DropdownActionComponent } from '@/pages/types';
import { useParams } from 'react-router';

type SwarmDataResource = SwarmConfigView | SwarmSecretView;
type ResourceKind = 'config' | 'secret';

const MonacoEditor = lazy(async () => {
  const module = await import('@/lib/monaco');
  return { default: module.MonacoEditor };
});

export const SwarmResourceEditDropdownAction = ({
  resource,
  onAction,
}: React.ComponentProps<DropdownActionComponent<SwarmDataResource>>) => (
  <DropdownActionButton
    title="Edit"
    icon={<Pencil className="h-4 w-4" />}
    disabled={resource.isStale || !hasCapability(resource, 'canWrite')}
    onClick={() => onAction?.('edit')}
  />
);

export const SwarmResourceEditInfoAction = ({
  resource,
  kind,
}: {
  resource: SwarmDataResource;
  kind: ResourceKind;
}) => {
  const [editing, setEditing] = useState<SwarmDataResource | null>(null);

  return (
    <>
      <Button
        variant="outline"
        size="sm"
        disabled={resource.isStale || !hasCapability(resource, 'canWrite')}
        className="min-h-9"
        onClick={() => setEditing(resource)}>
        <Pencil className="h-3.5 w-3.5" /> Edit
      </Button>
      {editing && (
        <SwarmResourceEditDialog
          resource={editing}
          kind={kind}
          open
          onOpenChange={(open) => !open && setEditing(null)}
        />
      )}
    </>
  );
};

export const SwarmResourceEditDialog = ({
  resource,
  kind,
  open,
  onOpenChange,
}: {
  resource: SwarmDataResource;
  kind: ResourceKind;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [labels, setLabels] = useState<KVPair[]>(() =>
    Object.entries(resource.labels).map(([key, value]) => ({ key, value })),
  );
  const configMutation = useMutate('updateSwarmConfigLabels');
  const secretMutation = useMutate('updateSwarmSecretLabels');
  const canInspect = hasCapability(resource, 'canInspect');
  const contentQuery = useRead(
    'getSwarmConfigData',
    { platformId, resourceId: resource.id },
    { enabled: open && kind === 'config' && canInspect },
  );
  const normalized = labels.map(({ key, value }) => ({ key: key.trim(), value }));
  const keys = normalized.map(({ key }) => key);
  const labelsInvalid = keys.some((key) => !key) || new Set(keys).size !== keys.length;
  const pending = configMutation.isPending || secretMutation.isPending;
  const problem = (contentQuery.error as { error?: ProblemDetails } | undefined)?.error;

  const save = async () => {
    const data = {
      versionIndex: resource.versionIndex,
      labels: Object.fromEntries(normalized.map(({ key, value }) => [key, value])),
    };

    if (kind === 'config') await configMutation.mutateAsync({ platformId, resourceId: resource.id, data });
    else await secretMutation.mutateAsync({ platformId, resourceId: resource.id, data });

    toast.success(`${kind === 'config' ? 'Config' : 'Secret'} updated.`);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>Edit {kind === 'config' ? 'config' : 'secret'}</DialogTitle>
          <DialogDescription>
            {kind === 'config'
              ? 'Config data is immutable and shown read-only. Labels can be changed.'
              : 'Docker never returns the stored secret value. Only labels can be changed.'}
          </DialogDescription>
        </DialogHeader>

        {kind === 'config' && (
          <div className="flex w-full min-w-0 flex-col gap-2">
            <span className="text-sm font-medium">Data</span>
            {!canInspect && (
              <AlertMessage title="Config data unavailable" type="info">
                Inspect permission is required to view this Config&apos;s data.
              </AlertMessage>
            )}
            {problem && (
              <AlertMessage title={problem.title ?? 'Unable to load config data'} type="error">
                {problem.detail ?? 'Docker could not return this Config data.'}
              </AlertMessage>
            )}
            {canInspect && !problem && (
              <div className="w-full min-w-0 overflow-hidden rounded-md border bg-muted/20">
                <Suspense fallback={<div className="h-55" />}>
                  <MonacoEditor
                    value={contentQuery.data?.data.content ?? ''}
                    language="plaintext"
                    readOnly
                    minHeight={220}
                    className="m-0"
                  />
                </Suspense>
              </div>
            )}
          </div>
        )}

        <div className="min-w-0">
          <KeyValuePairInput label="Labels" value={labels} onChange={setLabels} addButtonLabel="Add label" />
        </div>
        {labelsInvalid && <p className="text-xs text-destructive">Label keys must be non-empty and unique.</p>}

        <DialogFooter className="w-full flex-row justify-end">
          <Button onClick={save} disabled={labelsInvalid || pending}>
            <Save className="h-3.5 w-3.5" /> Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
