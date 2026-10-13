import { Database, FolderOpen } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Link, useLocation } from 'react-router';
import { ResourceBackupIndicator } from './resource-backup-indicator';
import { ResourceBackupSummaryView, ResourceBackupVolumeView } from '@/api/generated/api.types';
import { useRef } from 'react';
import { truncate } from '@/lib/truncate';

function storageCoverage(coverage?: ResourceBackupSummaryView | null, volume?: ResourceBackupVolumeView) {
  if (!coverage || !volume) return undefined;
  return {
    ...coverage,
    status: volume.status,
    volumes: [volume],
    policies: volume.policies,
    hasResourcePolicy: false,
    warnings: [],
  };
}

export function VolumeBackupLinks({
  volumes,
  platformId,
  dockerNodeId,
  coverage,
}: {
  volumes: string[];
  coverage?: ResourceBackupSummaryView | null;
  platformId: string;
  dockerNodeId?: string | null;
}) {
  const location = useLocation();
  const returnTo = `${location.pathname}${location.search}${location.hash}`;
  return (
    <div className="flex flex-col gap-2">
      {volumes.map((name) => {
        const item = coverage?.volumes.find(
          (v) => !v.hostPath && v.name === name && (v.dockerNodeId ?? null) === (dockerNodeId ?? null),
        );
        return (
          <span key={name} className="inline-flex min-w-0 max-w-full items-center gap-1.5">
            <Database className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
            <Link
              className="table-link truncate"
              title={name}
              aria-label={name}
              to={`/platforms/${platformId}/volumes/${encodeURIComponent(name)}${dockerNodeId ? `?dockerNodeId=${encodeURIComponent(dockerNodeId)}` : ''}`}>
              {truncate(name, 32)}
            </Link>
            <ResourceBackupIndicator
              compact
              source={{ $type: 'DockerVolume', platformId, volumeName: name, dockerNodeId }}
              coverage={storageCoverage(coverage, item)}
            />
            {item?.policies[0] && (
              <Link
                className="shrink-0 text-xs font-normal text-muted-foreground underline underline-offset-4 hover:text-foreground"
                aria-label={`Manage backup for ${name}`}
                to={`/backup-policies/edit/${item.policies[0].id}?${new URLSearchParams({ returnTo })}`}>
                Manage
              </Link>
            )}
          </span>
        );
      })}
    </div>
  );
}

export function BindMountBackupLinks({ coverage }: { coverage: ResourceBackupSummaryView }) {
  const dialogTitle = useRef<HTMLHeadingElement>(null);
  const location = useLocation();
  const returnTo = `${location.pathname}${location.search}${location.hash}`;
  const mounts = coverage.volumes.filter((volume) => volume.hostPath);
  const renderMount = (volume: (typeof mounts)[number], expanded = false) => (
    <div key={JSON.stringify([volume.dockerNodeId, volume.hostPath])} className="min-w-0 space-y-1.5">
      <div className="flex min-w-0 items-center gap-2">
        <FolderOpen className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <span
          className={expanded ? 'min-w-0 break-all text-xs font-normal' : 'min-w-0 truncate text-xs font-normal'}
          title={volume.hostPath!}>
          {volume.hostPath}
        </span>
        <ResourceBackupIndicator
          compact
          source={{
            $type: 'BindMount',
            platformId: coverage.platformId,
            hostPath: volume.hostPath!,
            dockerNodeId: volume.dockerNodeId,
          }}
          coverage={storageCoverage(coverage, volume)}
        />
        {!expanded && volume.policies[0] && (
          <Link
            className="shrink-0 text-xs font-normal text-muted-foreground underline underline-offset-4 hover:text-foreground"
            aria-label={`Manage backup for ${volume.hostPath}`}
            to={`/backup-policies/edit/${volume.policies[0].id}?${new URLSearchParams({ returnTo })}`}>
            Manage
          </Link>
        )}
      </div>
      {expanded && (
        <div className="flex flex-wrap gap-x-3 gap-y-1 pl-5 text-xs font-normal text-muted-foreground">
          {volume.dockerNodeId && <span>{volume.nodeHostname ?? volume.dockerNodeId}</span>}
          {volume.policies.map((policy) => (
            <Link
              key={policy.id}
              className="underline underline-offset-4 hover:text-foreground"
              to={`/backup-policies/edit/${policy.id}?${new URLSearchParams({ returnTo })}`}>
              {policy.name}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
  return (
    <div className="min-w-0 space-y-2">
      {mounts.slice(0, 2).map((volume) => renderMount(volume))}
      {mounts.length > 2 && (
        <Dialog>
          <DialogTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="h-auto px-0 py-0.5 text-xs font-normal text-muted-foreground hover:bg-transparent hover:text-foreground">
              View all {mounts.length} bind mounts
            </Button>
          </DialogTrigger>
          <DialogContent
            className="sm:max-w-xl"
            onOpenAutoFocus={(event) => {
              event.preventDefault();
              dialogTitle.current?.focus();
            }}>
            <DialogHeader>
              <DialogTitle ref={dialogTitle} tabIndex={-1} className="outline-none">
                Bind mounts
              </DialogTitle>
              <DialogDescription>Host paths mounted by this container and their backup coverage.</DialogDescription>
            </DialogHeader>
            <div className="max-h-[60vh] space-y-4 overflow-y-auto pr-2">
              {mounts.map((volume) => renderMount(volume, true))}
            </div>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}
