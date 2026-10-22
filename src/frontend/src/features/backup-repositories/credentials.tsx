import { BackupRepositoryView } from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { useMutate } from '@/lib/hooks';
import { notifyRequestError, requestErrorMessage } from '@/lib/request-error';
import { useQueryClient } from '@tanstack/react-query';
import { AlertCircle, ChevronDown, KeyRound, LoaderCircle, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import { getRepositoryOperationContext, SecretSelector } from './form/form';

type Credentials = {
  accessKey: string;
  secretKey: string;
  sessionToken: string | null;
};

export function RepositoryCredentials({
  repository,
  disabled = false,
  onUpdated,
  onPendingChange,
}: {
  repository: BackupRepositoryView;
  disabled?: boolean;
  onUpdated?: (repository: BackupRepositoryView) => void;
  onPendingChange?: (pending: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const mutation = useMutate('updateBackupRepositoryCredentials');
  // Untouched selections come directly from the latest repository. Keep a snapshot
  // only while editing so a refresh cannot silently overwrite another rotation.
  const [draft, setDraft] = useState<{ base: string; values: Credentials } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const spec = repository.spec;
  if (spec.$type !== 'S3Compatible') return null;
  const current: Credentials = {
    accessKey: spec.accessKeySecretId,
    secretKey: spec.secretKeySecretId,
    sessionToken: spec.sessionTokenSecretId ?? null,
  };
  const currentKey = JSON.stringify(current);
  const values = draft?.values ?? current;
  const { accessKey, secretKey, sessionToken } = values;
  const permitted = repository.capabilities?.canWrite && repository.capabilities?.canExecute;
  const changed = JSON.stringify(values) !== currentKey;
  const stale = changed && draft !== null && draft.base !== currentKey;
  const blocked = disabled || !permitted || mutation.isPending;
  const select = (patch: Partial<Credentials>) => {
    const next = { ...values, ...patch };
    setDraft(
      JSON.stringify(next) === currentKey
        ? null
        : { base: changed ? (draft?.base ?? currentKey) : currentKey, values: next },
    );
    setError(null);
  };
  const reset = () => {
    setDraft(null);
    setError(null);
  };
  const save = async () => {
    if (blocked || stale || !changed || !accessKey || !secretKey) return;
    setError(null);
    onPendingChange?.(true);
    try {
      const response = await mutation.mutateAsync({
        id: repository.id,
        data: {
          accessKeySecretId: accessKey,
          secretKeySecretId: secretKey,
          sessionTokenSecretId: sessionToken,
          ...getRepositoryOperationContext(repository),
        },
      });
      onUpdated?.(response.data);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] }),
        queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id: repository.id }] }),
      ]);
      toast.success('S3 credentials validated and updated');
      reset();
      setOpen(false);
    } catch (error) {
      setError(requestErrorMessage(error));
      notifyRequestError(error);
    } finally {
      onPendingChange?.(false);
    }
  };
  return (
    <details open={open} className="group rounded-lg border">
      <summary
        onClick={(event) => {
          event.preventDefault();
          setOpen(!open);
        }}
        className="flex cursor-pointer list-none items-center gap-3 rounded-lg p-4 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring [&::-webkit-details-marker]:hidden">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
          <KeyRound aria-hidden="true" className="size-4" />
        </span>
        <span className="min-w-0 flex-1 space-y-1">
          <span className="block font-medium">Update S3 credentials</span>
          <span className="block text-xs text-muted-foreground">Replace the secrets used to access your storage.</span>
        </span>
        <ChevronDown
          aria-hidden="true"
          className="size-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180 motion-reduce:transition-none"
        />
      </summary>
      {open && (
        <div className="space-y-4 border-t p-4">
          <p className="text-xs leading-relaxed text-muted-foreground">
            Select secrets from Bindings. The destination and encryption password stay the same.
          </p>
          {!permitted && <p className="text-xs text-muted-foreground">Write and Execute permissions are required.</p>}
          {disabled && <p className="text-xs text-muted-foreground">Finish or save your repository changes first.</p>}
          <div role="group" aria-label="Replacement S3 credentials" className="grid gap-4 sm:grid-cols-2">
            <div role="group" aria-label="Access key secret" className="min-w-0 space-y-2">
              <div className="text-sm font-medium">Access key secret</div>
              <SecretSelector
                value={accessKey}
                onChange={(value) => select({ accessKey: value ?? '' })}
                disabled={blocked || stale}
                targetResourceId={repository.id}
              />
            </div>
            <div role="group" aria-label="Secret key secret" className="min-w-0 space-y-2">
              <div className="text-sm font-medium">Secret key secret</div>
              <SecretSelector
                value={secretKey}
                onChange={(value) => select({ secretKey: value ?? '' })}
                disabled={blocked || stale}
                targetResourceId={repository.id}
              />
            </div>
            <div role="group" aria-label="Session token secret" className="min-w-0 space-y-2 sm:col-span-2">
              <div className="text-sm font-medium">
                Session token secret <span className="font-normal text-muted-foreground">(optional)</span>
              </div>
              <SecretSelector
                value={sessionToken}
                onChange={(value) => select({ sessionToken: value })}
                allowNone
                disabled={blocked || stale}
                targetResourceId={repository.id}
              />
              <p className="text-xs text-muted-foreground">Only needed for temporary storage credentials.</p>
            </div>
          </div>
          {stale && (
            <Alert>
              <AlertCircle aria-hidden="true" />
              <AlertTitle className="line-clamp-none">Credentials changed while you were editing</AlertTitle>
              <AlertDescription>
                Reset selections to load the current credentials before making changes.
              </AlertDescription>
            </Alert>
          )}
          {error && (
            <Alert variant="destructive">
              <AlertCircle aria-hidden="true" />
              <AlertTitle>Could not update credentials</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          <div className="flex items-start gap-2 rounded-md bg-muted/50 p-3 text-xs leading-relaxed text-muted-foreground">
            <ShieldCheck aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
            <p>
              Citadel checks access from Core before saving. Your current credentials remain active if validation fails.
            </p>
          </div>
          <div
            className="flex flex-col gap-2 border-t pt-4 sm:flex-row sm:items-center sm:justify-end"
            aria-live="polite">
            {changed && (
              <Button type="button" variant="ghost" onClick={reset} disabled={mutation.isPending}>
                Reset selections
              </Button>
            )}
            <Button type="button" onClick={save} disabled={blocked || stale || !changed || !accessKey || !secretKey}>
              {mutation.isPending && (
                <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />
              )}
              {mutation.isPending ? 'Validating credentials…' : 'Validate and update'}
            </Button>
          </div>
        </div>
      )}
    </details>
  );
}
