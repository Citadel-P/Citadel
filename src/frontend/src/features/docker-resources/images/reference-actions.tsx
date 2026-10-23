import { useId, useState } from 'react';
import { ArrowRight, Download, KeyRound, RefreshCw, Server } from 'lucide-react';
import { Link } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import type {
  ImageCapabilitiesView,
  ImageRegistryView,
  ImageTagUpdate,
  AuthorizedRegistryView,
} from '@/api/generated/api.types';
import { LookupResourceType } from '@/api/generated/api.types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { ResourceSelectorField } from '@/components/custom/common';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { DetailSection } from '@/components/custom/resource-detail';
import { useAppContext } from '@/lib/context/app-context';
import { useMutate } from '@/lib/hooks';
import { notifyRequestError } from '@/lib/request-error';
import { useTaskSheet } from '@/lib/atoms';
import { ImageUpdateStatus } from './update-status';

export type ImageReferenceTarget = {
  dockerImageId: string;
  tags: string[];
  dockerNodeId?: string | null;
  isStale?: boolean;
  capabilities?: ImageCapabilitiesView | null;
  registry?: ImageRegistryView | null;
};
export type ImageReferenceAction = 'check' | 'pull';
function unavailable(image: ImageReferenceTarget | undefined, kind: ImageReferenceAction) {
  return (
    !image ||
    image.isStale ||
    !!image.dockerNodeId ||
    !image.capabilities?.canRead ||
    (kind === 'pull' && (!image.capabilities.canPull || !!image.dockerNodeId)) ||
    imageTags(image).length === 0
  );
}
function imageTags(image: ImageReferenceTarget) {
  return [...new Set(image.tags.filter((tag) => tag.trim() && !tag.includes('@') && !tag.includes('<')))];
}
function useImagePullAction() {
  const { currentPlatform } = useAppContext();
  const { open } = useTaskSheet('Image');
  const canPull = currentPlatform?.capabilities?.canRead === true && currentPlatform.capabilities.canPull;
  return {
    canPull,
    start(image: ImageReferenceTarget, reference: string, registryId?: string) {
      if (!canPull || !currentPlatform?.id || unavailable(image, 'pull') || !imageTags(image).includes(reference))
        return;
      open({
        kind: 'pull',
        payload: {
          platformId: currentPlatform.id,
          sourceImageId: image.dockerImageId,
          imageTag: reference,
          ...(registryId ? { registryId } : {}),
        },
      });
    },
  };
}
export function ImageReferenceButton({
  resources,
  kind,
}: {
  resources: ImageReferenceTarget[];
  kind: ImageReferenceAction;
}) {
  const [open, setOpen] = useState(false);
  const image = resources[0];
  const pull = useImagePullAction();
  return (
    <>
      <ActionButton
        variant="outline"
        title={kind === 'check' ? 'Check for updates' : 'Pull latest'}
        icon={kind === 'check' ? <RefreshCw className="h-4 w-4" /> : <Download className="h-4 w-4" />}
        iconPosition="left"
        disabled={resources.length !== 1 || unavailable(image, kind) || (kind === 'pull' && !pull.canPull)}
        disabledReason={
          image?.dockerNodeId
            ? 'This action is unavailable for node-local images.'
            : image && imageTags(image).length === 0
              ? 'This image has no repository tag to pull or check.'
              : undefined
        }
        onClick={() => {
          const tags = imageTags(image);
          if (kind === 'pull' && tags.length === 1) pull.start(image, tags[0]);
          else setOpen(true);
        }}
      />
      {open && <ImageReferenceDialog image={image} kind={kind} onClose={() => setOpen(false)} />}
    </>
  );
}
export function ImageReferenceDropdown({
  resource,
  kind,
  onAction,
}: {
  resource: ImageReferenceTarget;
  kind: ImageReferenceAction;
  onAction?: (key: string) => void;
}) {
  const pull = useImagePullAction();
  return (
    <DropdownActionButton
      title={kind === 'check' ? 'Check for updates' : 'Pull latest'}
      icon={kind === 'check' ? <RefreshCw className="h-4 w-4" /> : <Download className="h-4 w-4" />}
      disabled={unavailable(resource, kind) || (kind === 'pull' && !pull.canPull)}
      onClick={() => {
        const tags = imageTags(resource);
        if (kind === 'pull' && tags.length === 1) pull.start(resource, tags[0]);
        else onAction?.(kind === 'check' ? 'checkImage' : 'pullImage');
      }}
    />
  );
}
export function ImageReferenceDialog({
  image,
  kind,
  onClose,
}: {
  image: ImageReferenceTarget;
  kind: ImageReferenceAction;
  onClose: () => void;
}) {
  const tagInputId = useId();
  const registryInputId = useId();
  const tags = imageTags(image);
  const pull = useImagePullAction();
  const [reference, setReference] = useState(tags.length === 1 ? tags[0] : '');
  const [registry, setRegistry] = useState<Pick<AuthorizedRegistryView, 'id' | 'name'> | undefined>(
    image.registry ?? undefined,
  );
  const [result, setResult] = useState<ImageTagUpdate>();
  const { currentPlatform } = useAppContext();
  const [platformId] = useState(currentPlatform?.id);
  const queryClient = useQueryClient();
  const check = useMutate('checkImageUpdate');
  const changed = platformId !== currentPlatform?.id;
  async function submit() {
    if (!reference || !registry || !platformId || changed) return;
    setResult(undefined);
    try {
      const response = await check.mutateAsync({
        data: {
          platformId,
          dockerImageId: image.dockerImageId,
          dockerNodeId: image.dockerNodeId,
          registryId: registry.id,
          reference,
        },
      });
      setResult(response.data);
      await Promise.all(
        ['listImages', 'inspectImage'].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
      );
    } catch (error) {
      notifyRequestError(error);
    }
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !check.isPending) onClose();
      }}>
      <DialogContent className="max-h-[85vh] overflow-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{kind === 'check' ? 'Check for updates' : 'Pull latest for this tag'}</DialogTitle>
          <DialogDescription>
            {kind === 'check'
              ? 'Compare this host’s image with the selected registry. This check does not download or deploy anything.'
              : 'Downloads the image to this host. Does not redeploy workloads.'}
          </DialogDescription>
        </DialogHeader>
        <div className="flex min-w-0 items-start gap-2 rounded-md bg-muted/30 px-3 py-2 text-sm">
          <Server className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span className="shrink-0 text-muted-foreground">{kind === 'check' ? 'Host' : 'Destination'}</span>
          <span className="min-w-0 break-words font-medium">
            {currentPlatform?.name}
            {image.dockerNodeId ? ` / ${image.dockerNodeId}` : ''}
          </span>
        </div>
        {changed ? (
          <p role="alert">The Platform changed. Close this dialog and select the image again.</p>
        ) : (
          <>
            <div className="min-w-0 space-y-2">
              <Label htmlFor={tagInputId}>Image tag</Label>
              <ResourceSelectorField
                id={tagInputId}
                className="max-w-none"
                items={tags.map((tag) => ({ id: tag, name: tag }))}
                selected={reference}
                placeholder="Select a tag"
                searchPlaceholder="Search image tags"
                allowClear={false}
                disabled={check.isPending}
                onSelect={(tag) => {
                  setReference(tag?.id ?? '');
                  setResult(undefined);
                }}
              />
            </div>
            {kind === 'pull' ? (
              <Button
                disabled={!reference || !pull.canPull || unavailable(image, 'pull')}
                onClick={() => {
                  if (changed) return;
                  pull.start(image, reference);
                  onClose();
                }}>
                Pull latest
              </Button>
            ) : (
              <>
                <DetailSection
                  title="Registry credentials"
                  description="Choose a Registry that matches this image’s host and has access to the repository."
                  icon={KeyRound}>
                  <div className="min-w-0 space-y-2">
                    <Label htmlFor={registryInputId}>Registry</Label>
                    <ResourceSelectorField
                      id={registryInputId}
                      className="max-w-none"
                      sourceType={LookupResourceType.Image}
                      targetType={LookupResourceType.Registry}
                      selected={registry}
                      onSelect={(registry) => {
                        setRegistry(registry);
                        setResult(undefined);
                      }}
                      placeholder="Select a Registry"
                      disabled={check.isPending}
                    />
                  </div>
                  <div className="flex flex-col gap-2 border-t pt-3 sm:flex-row sm:items-center sm:justify-between">
                    <p className="text-xs leading-relaxed text-muted-foreground">
                      Only images published to a Registry can be checked.
                    </p>
                    <Button
                      variant="link"
                      size="sm"
                      className="h-auto shrink-0 self-start p-0 text-xs"
                      disabled={check.isPending}
                      asChild>
                      <Link
                        to="/registries"
                        aria-disabled={check.isPending}
                        onClick={(event) => {
                          if (check.isPending) event.preventDefault();
                          else onClose();
                        }}>
                        Manage Registries
                        <ArrowRight className="size-3.5" aria-hidden="true" />
                      </Link>
                    </Button>
                  </div>
                </DetailSection>
                <div aria-live="polite" aria-atomic="true">
                  {result && (
                    <div className="space-y-3 rounded-lg border bg-muted/15 p-4">
                      <h2 className="text-sm font-semibold">Check result</h2>
                      <ImageUpdateStatus updates={[result]} detail />
                    </div>
                  )}
                </div>
                <DialogFooter>
                  <Button variant="outline" disabled={check.isPending} onClick={onClose}>
                    Close
                  </Button>
                  <Button disabled={!reference || !registry || check.isPending} onClick={() => void submit()}>
                    <RefreshCw
                      className={check.isPending ? 'size-4 animate-spin motion-reduce:animate-none' : 'size-4'}
                      aria-hidden="true"
                    />
                    {check.isPending ? 'Checking…' : result ? 'Check again' : 'Check for updates'}
                  </Button>
                  {result?.status === 'UpdateAvailable' && !result.isStale && (
                    <Button
                      disabled={check.isPending || !pull.canPull || unavailable(image, 'pull')}
                      onClick={() => {
                        if (changed || check.isPending) return;
                        pull.start(image, reference, registry?.id);
                        onClose();
                      }}>
                      <Download className="size-4" aria-hidden="true" />
                      Pull latest
                    </Button>
                  )}
                </DialogFooter>
              </>
            )}
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
