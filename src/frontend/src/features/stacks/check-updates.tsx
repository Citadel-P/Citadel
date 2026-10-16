import { useId, useState } from 'react';
import { ArrowRight, KeyRound, RefreshCw } from 'lucide-react';
import { Link } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import {
  LookupResourceType,
  StackSource,
  type AuthorizedRegistryView,
  type StackView,
} from '@/api/generated/api.types';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { ResourceSelectorField } from '@/components/custom/common';
import { DetailSection } from '@/components/custom/resource-detail';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useMutate } from '@/lib/hooks';
import { hasCapabilities } from '@/lib/resource-capabilities';
import { notifyRequestError, requestErrorMessage } from '@/lib/request-error';
import { useStackUpdateCheck } from './actions';

export function StackUpdateCheckButton({ resource }: { resource: StackView }) {
  const [setupStack, setSetupStack] = useState<StackView>();
  const check = useStackUpdateCheck(resource);
  const permitted = hasCapabilities(resource, ['canWrite']);
  // Unpinned Git stacks can still check their source without image credentials.
  const needsRegistry =
    !resource.spec?.registryId &&
    (resource.stackSource === StackSource.WebEditor ||
      (resource.spec?.$type === 'Git' && !!resource.spec.commitSha?.trim()));
  return (
    <>
      <ActionButton
        title="Check for updates"
        icon={<RefreshCw className="size-4" />}
        iconPosition="left"
        variant="outline"
        disabled={!permitted || !check.canExecute}
        disabledReason={check.disabledReason}
        loading={check.isPending}
        onClick={() => {
          if (!permitted || !check.canExecute) return;
          if (needsRegistry) setSetupStack(resource);
          else void check.run().catch(notifyRequestError);
        }}
      />
      {setupStack && (
        <StackRegistryCheckDialog
          key={setupStack.id}
          stack={setupStack}
          permitted={permitted && check.canExecute}
          onClose={() => setSetupStack(undefined)}
        />
      )}
    </>
  );
}

function StackRegistryCheckDialog({
  stack,
  permitted,
  onClose,
}: {
  stack: StackView;
  permitted: boolean;
  onClose: () => void;
}) {
  const inputId = useId();
  const [registry, setRegistry] = useState<Pick<AuthorizedRegistryView, 'id' | 'name'>>();
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const update = useMutate('updateStack');
  const check = useStackUpdateCheck(stack);
  const queryClient = useQueryClient();

  async function submit() {
    if (!registry || busy || !permitted) return;
    setBusy(true);
    setError(undefined);
    try {
      if (!saved) {
        await update.mutateAsync({
          id: stack.id,
          data: { rowVersion: stack.rowVersion, spec: { registryId: registry.id } },
        });
        setSaved(true);
        await Promise.all(
          ['getStack', 'getStackConfig', 'listStacks'].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
        );
      }
      await check.run();
      onClose();
    } catch (failure) {
      setError(requestErrorMessage(failure));
      // Keep the saved Registry if the subsequent check fails; retry only the check.
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}>
      <DialogContent className="max-h-[85vh] overflow-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Check for updates</DialogTitle>
          <DialogDescription>
            Select a Registry to check image updates for {stack.name}. This check does not download or deploy anything.
          </DialogDescription>
        </DialogHeader>
        <DetailSection
          title="Registry credentials"
          description="Choose a Registry that matches the service images and has access to their repositories."
          icon={KeyRound}>
          <div className="min-w-0 space-y-2">
            <Label htmlFor={inputId}>Registry</Label>
            <ResourceSelectorField
              id={inputId}
              sourceType={LookupResourceType.Stack}
              targetType={LookupResourceType.Registry}
              sourceResourceId={stack.id}
              selected={registry}
              onSelect={setRegistry}
              disabled={busy || saved || !permitted}
              placeholder="Select a Registry"
              className="max-w-none"
            />
          </div>
          <p className="text-xs leading-relaxed text-muted-foreground">
            This saves the Registry in the stack configuration for future image pulls and update checks.
          </p>
          <div className="border-t pt-3">
            <Button variant="link" size="sm" className="h-auto p-0 text-xs" disabled={busy} asChild>
              <Link
                to="/registries"
                aria-disabled={busy}
                onClick={(event) => {
                  if (busy) event.preventDefault();
                  else onClose();
                }}>
                Manage Registries <ArrowRight className="size-3.5" aria-hidden="true" />
              </Link>
            </Button>
          </div>
        </DetailSection>
        {saved && !busy && (
          <p role="status" className="text-sm text-muted-foreground">
            Registry saved. You can retry the update check.
          </p>
        )}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
        {!permitted && (
          <p role="alert" className="text-sm text-destructive">
            The stack changed or you no longer have permission to check it. Close this dialog and try again.
          </p>
        )}
        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={!registry || busy || !permitted} onClick={() => void submit()}>
            <RefreshCw
              className={busy ? 'size-4 animate-spin motion-reduce:animate-none' : 'size-4'}
              aria-hidden="true"
            />
            {busy ? 'Checking…' : saved ? 'Retry check' : 'Save Registry and check'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
