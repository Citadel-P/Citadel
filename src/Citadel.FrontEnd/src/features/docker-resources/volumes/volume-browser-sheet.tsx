import { useApiClientContext } from '@/api/api-client-context';
import { DockerVolumeResultView, PlatformStatus, ProblemDetails, VolumeFileEntryType } from '@/api/generated/api.types';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { AlertMessage } from '@/components/custom/alert-message';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { BrowserEntry, FileTree } from '@/components/custom/file-browser';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useAppContext } from '@/lib/context/app-context';
import { hasCapability, hasCapabilities } from '@/lib/resource-capabilities';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { cn } from '@/lib/utils';
import { ButtonGroupComponent, DropdownActionComponent, ButtonActionComponent } from '@/pages/types';
import { Archive, Download, FolderOpen, HardDrive, RefreshCw } from 'lucide-react';
import { useCallback, useState } from 'react';
import { toast } from 'sonner';

const ROOT_PATH = '/';

type VolumeBrowserSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  volume?: DockerVolumeResultView | null;
  platformId?: string;
};

export function VolumeBrowserSheet({ open, onOpenChange, volume, platformId }: VolumeBrowserSheetProps) {
  const { currentPlatform } = useAppContext();
  const effectivePlatformId = platformId ?? currentPlatform?.id ?? '';
  const isOffline = currentPlatform?.id === effectivePlatformId && currentPlatform?.status === PlatformStatus.Offline;
  const [treeVersion, setTreeVersion] = useState(0);

  const description = [volume?.driver, volume?.inUse ? 'In use' : null].filter(Boolean).join(' - ');

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-[min(92vw,720px)] sm:max-w-none">
        <SheetHeader className="border-b pr-10">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <SheetTitle className="flex min-w-0 items-center gap-2">
                <HardDrive className="size-4 shrink-0 text-muted-foreground" />
                <span className="truncate">{volume?.name ?? volume?.id ?? 'Volume'}</span>
              </SheetTitle>
              {description && <SheetDescription>{description}</SheetDescription>}
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={!volume || !effectivePlatformId || isOffline}
              onClick={() => setTreeVersion((value) => value + 1)}>
              <RefreshCw className="size-4" />
              Refresh
            </Button>
          </div>
        </SheetHeader>

        <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
          {isOffline ? (
            <AlertMessage type="warning" title="Platform offline">
              Volume files cannot be browsed while the platform is disconnected.
            </AlertMessage>
          ) : volume && effectivePlatformId ? (
            <VolumeTree
              key={`${effectivePlatformId}:${volume.id}:${treeVersion}`}
              platformId={effectivePlatformId}
              volumeName={volume.id}
              enabled={open}
            />
          ) : (
            <div className="py-8 text-sm text-muted-foreground">No volume selected.</div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

export const VolumeBrowseDropdownAction: DropdownActionComponent<DockerVolumeResultView> = ({ resource, onAction }) => {
  const canBrowse = hasCapability(resource, 'canBrowse');

  return (
    <DropdownActionButton
      title="Browse"
      icon={<FolderOpen className="size-4" />}
      variant="outline"
      disabled={!canBrowse}
      onClick={() => onAction?.('browse')}
    />
  );
};

export const VolumeBrowseGroupAction: ButtonGroupComponent<DockerVolumeResultView> = ({ resources }) => {
  const { currentPlatform } = useAppContext();
  const [volume, setVolume] = useState<DockerVolumeResultView | null>(null);
  const selected = resources[0];
  const canBrowse =
    resources.length === 1 &&
    !!selected &&
    hasCapabilities(selected, ['canBrowse']) &&
    currentPlatform?.status !== PlatformStatus.Offline;

  return (
    <>
      <ActionButton
        title="Browse"
        iconPosition="left"
        variant="outline"
        icon={<FolderOpen className="size-4" />}
        disabled={!canBrowse}
        onClick={() => {
          if (canBrowse) setVolume(selected);
        }}
      />
      <VolumeBrowserSheet
        open={!!volume}
        onOpenChange={(open) => !open && setVolume(null)}
        volume={volume}
        platformId={currentPlatform?.id}
      />
    </>
  );
};

export const VolumeBrowseInfoAction: ButtonActionComponent<DockerVolumeResultView> = ({ resource }) => {
  const { currentPlatform } = useAppContext();
  const [open, setOpen] = useState(false);
  const canBrowse = hasCapability(resource, 'canBrowse') && currentPlatform?.status !== PlatformStatus.Offline;

  return (
    <>
      <ActionButton
        title="Browse"
        iconPosition="left"
        variant="outline"
        icon={<FolderOpen className="size-4" />}
        disabled={!canBrowse}
        onClick={() => setOpen(true)}
      />
      <VolumeBrowserSheet open={open} onOpenChange={setOpen} volume={resource} platformId={currentPlatform?.id} />
    </>
  );
};

function VolumeTree({ platformId, volumeName, enabled }: { platformId: string; volumeName: string; enabled: boolean }) {
  const { apiClient } = useApiClientContext();
  const formatDateTime = useProfileDateTimeFormatter();
  const [downloadingPath, setDownloadingPath] = useState<string | null>(null);

  const downloadPath = useCallback(
    async (path: string, entryType: VolumeFileEntryType) => {
      const normalized = normalizeBrowserPath(path);
      if (normalized === ROOT_PATH || !canDownloadEntry(entryType)) return;

      setDownloadingPath(normalized);
      try {
        const response = await apiClient.request<Response, ProblemDetails>({
          path: `/api/v1/platforms/${encodeURIComponent(platformId)}/volumes/${encodeURIComponent(volumeName)}/files/download`,
          method: 'GET',
          query: { path: normalized },
          secure: true,
          headers: { Accept: 'application/octet-stream, application/x-tar' },
        });

        const blob = await response.blob();
        const fileName =
          getContentDispositionFileName(response.headers.get('content-disposition')) ??
          getDownloadFallbackName(normalized, entryType);

        const url = URL.createObjectURL(blob);
        const anchor = document.createElement('a');
        anchor.href = url;
        anchor.download = fileName;
        document.body.appendChild(anchor);
        anchor.click();
        anchor.remove();
        URL.revokeObjectURL(url);
      } catch (error) {
        const problem = await readDownloadProblem(error);
        toast.error(problem.title, { description: problem.detail });
      } finally {
        setDownloadingPath(null);
      }
    },
    [apiClient, platformId, volumeName],
  );

  const loadDirectory = useCallback(
    async (path: string, signal: AbortSignal) => {
      const response = await apiClient.api.listVolumeDirectory(platformId, volumeName, { path }, { signal });
      return {
        entries: response.data.entries.map(
          (entry): BrowserEntry => ({
            name: entry.name,
            path: entry.path,
            type: mapVolumeEntryType(entry.type),
            size: entry.size,
            secondaryText: entry.linkTarget,
            metadataText: entry.modifiedAt ? formatDateTime(entry.modifiedAt) : null,
          }),
        ),
        isTruncated: response.data.isTruncated,
      };
    },
    [apiClient, formatDateTime, platformId, volumeName],
  );

  const renderEntryActions = useCallback(
    (entry: BrowserEntry) => (
      <NodeDownloadButton
        path={entry.path}
        type={mapBrowserEntryType(entry.type)}
        downloadingPath={downloadingPath}
        onDownload={downloadPath}
      />
    ),
    [downloadPath, downloadingPath],
  );

  return (
    <FileTree
      queryKey={['volume-browser', platformId, volumeName]}
      rootPath={ROOT_PATH}
      rootName="/"
      loadDirectory={loadDirectory}
      enabled={enabled}
      renderEntryActions={renderEntryActions}
      getErrorTitle={getVolumeBrowseErrorTitle}
    />
  );
}

function NodeDownloadButton({
  path,
  type,
  downloadingPath,
  onDownload,
}: {
  path: string;
  type: VolumeFileEntryType;
  downloadingPath: string | null;
  onDownload: (path: string, entryType: VolumeFileEntryType) => Promise<void>;
}) {
  const isPending = downloadingPath === path;
  const canDownload = canDownloadEntry(type);

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-xs"
      className="size-7 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:focus-visible:opacity-100"
      disabled={!canDownload || isPending}
      title={type === VolumeFileEntryType.Directory ? 'Download directory archive' : 'Download file'}
      onClick={() => onDownload(path, type)}>
      {type === VolumeFileEntryType.Directory ? (
        <Archive className={cn('size-3.5', isPending && 'animate-pulse')} />
      ) : (
        <Download className={cn('size-3.5', isPending && 'animate-pulse')} />
      )}
    </Button>
  );
}

function normalizeBrowserPath(path: string | null | undefined) {
  if (!path || path.trim() === '') return ROOT_PATH;
  const normalized = path.startsWith('/') ? path : `/${path}`;
  return normalized.replace(/\/+/g, '/').replace(/\/$/g, '') || ROOT_PATH;
}

function canDownloadEntry(type: VolumeFileEntryType) {
  return type === VolumeFileEntryType.File || type === VolumeFileEntryType.Directory;
}

function mapVolumeEntryType(type: VolumeFileEntryType): BrowserEntry['type'] {
  switch (type) {
    case VolumeFileEntryType.Directory:
      return 'directory';
    case VolumeFileEntryType.Symlink:
      return 'symlink';
    default:
      return 'file';
  }
}

function mapBrowserEntryType(type: BrowserEntry['type']): VolumeFileEntryType {
  switch (type) {
    case 'directory':
      return VolumeFileEntryType.Directory;
    case 'symlink':
      return VolumeFileEntryType.Symlink;
    default:
      return VolumeFileEntryType.File;
  }
}

function getDownloadFallbackName(path: string, entryType: VolumeFileEntryType) {
  const name = path.split('/').filter(Boolean).at(-1) ?? 'volume';
  return entryType === VolumeFileEntryType.Directory ? `${name}.tar` : name;
}

function getContentDispositionFileName(disposition: string | null) {
  if (!disposition) return null;

  const encodedMatch = disposition.match(/filename\*=UTF-8''([^;]+)/i);
  if (encodedMatch?.[1]) {
    try {
      return decodeURIComponent(encodedMatch[1].replace(/^"|"$/g, ''));
    } catch {
      return encodedMatch[1].replace(/^"|"$/g, '');
    }
  }

  const match = disposition.match(/filename="?([^";]+)"?/i);
  return match?.[1] ?? null;
}

async function readDownloadProblem(error: unknown): Promise<{ title: string; detail?: string }> {
  if (error instanceof Response) {
    const contentType = error.headers.get('content-type') ?? '';
    if (contentType.includes('application/json')) {
      const problem = (await error.json().catch(() => null)) as ProblemDetails | null;
      if (problem) {
        return {
          title: problem.title ?? `Download failed (${error.status})`,
          detail: problem.detail,
        };
      }
    }

    const text = await error.text().catch(() => '');
    return {
      title: `Download failed (${error.status})`,
      detail: text || error.statusText,
    };
  }

  const problem = getProblemDetails(error);
  if (problem) {
    return {
      title: problem.title ?? 'Download failed',
      detail: problem.detail,
    };
  }

  return {
    title: 'Download failed',
    detail: error instanceof Error ? error.message : undefined,
  };
}

function getProblemDetails(error: unknown): ProblemDetails | undefined {
  return (error as any)?.error as ProblemDetails | undefined;
}

function getVolumeBrowseErrorTitle(error: unknown) {
  const problem = getProblemDetails(error);
  if (!problem) return 'Volume browsing failed';
  if (problem.status === 403) return 'Permission denied';
  if (problem.status === 404) return 'Path not found';

  const message = `${problem.title ?? ''} ${problem.detail ?? ''}`.toLowerCase();
  if (message.includes('not supported') || message.includes('unsupported')) return 'Volume browsing unavailable';

  return problem.title ?? 'Volume browsing failed';
}
