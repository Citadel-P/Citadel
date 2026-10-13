import { BackupCoverageStatus, ResourceBackupSummaryView, BackupSourceSpec } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { Link, useLocation } from 'react-router';
import { CoverageIndicator } from './coverage-indicator';
import { VolumeTarget, backupConfigurationIssue, backupCoverageMessages } from './coverage';
import { backupConfigurationUrl } from './navigation';
import { cn } from '@/lib/utils';
import { ChevronDown, Database, FolderOpen, Info, TriangleAlert } from 'lucide-react';

const emptyStorageMessage = 'No supported volumes or bind mounts were resolved for this resource.';

export type ResourceBackupSource =
  | Exclude<BackupSourceSpec, { $type: 'CitadelSystem' }>
  | { $type: 'Container'; platformId: string; containerId: string };

export function ResourceBackupSummary({
  source,
  coverage,
  returnTo: returnPath,
}: {
  source: ResourceBackupSource;
  returnTo?: string;
  coverage?: ResourceBackupSummaryView | null;
}) {
  return (
    <>
      {!coverage || coverage.unavailable ? (
        <div className="flex items-start gap-3 rounded-lg bg-muted/40 p-3 text-muted-foreground">
          <Info className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <p className="text-sm leading-relaxed">Backup coverage is unavailable.</p>
        </div>
      ) : (
        <BackupSummaryDetails source={source} coverage={coverage} returnTo={returnPath} />
      )}
    </>
  );
}

export function BackupSummaryDetails({
  source,
  coverage,
  returnTo: returnPath,
}: {
  source: ResourceBackupSource;
  returnTo?: string;
  coverage: ResourceBackupSummaryView;
}) {
  const location = useLocation();
  const returnTo = returnPath ?? `${location.pathname}${location.search}${location.hash}`;
  const volumes = coverage.volumes;
  const inventoryIncomplete = coverage.reasons?.some((reason) => reason === 'InventoryIncomplete');
  const isWorkload = source.$type === 'Stack' || source.$type === 'Deployment' || source.$type === 'SwarmService';
  const hasBindMounts = volumes.some((volume) => volume.hostPath);
  const targetLabel = hasBindMounts ? 'storage targets' : 'volumes';
  const rows = volumes.map((volume) => ({ volume, coverage: { ...volume, status: volume.status ?? undefined } }));
  const excludedVolumes = volumes.filter((volume) => !volume.includedInResourcePolicy);
  const protectedCount = rows.filter((row) => row.coverage.status === BackupCoverageStatus.Protected).length;
  const unknownCount = rows.filter((row) => row.coverage.status === undefined).length;
  const existingWorkloadPolicy = coverage.hasResourcePolicy;
  const policies = coverage.policies;
  const lastSuccessful = rows
    .map((row) => row.coverage.lastSuccessfulRunAt)
    .filter((value): value is string => Boolean(value))
    .sort()
    .at(-1);
  return (
    <div className="space-y-4">
      <div
        className={cn(
          'flex flex-wrap items-start justify-between gap-3',
          !volumes.length && 'rounded-lg bg-muted/40 p-4',
        )}>
        <div className="min-w-0 space-y-2">
          <p className={cn('text-sm font-medium', !volumes.length && 'text-muted-foreground')} role="status">
            {volumes.length
              ? `${protectedCount} of ${volumes.length} ${targetLabel} protected`
              : inventoryIncomplete
                ? 'Storage inventory incomplete'
                : 'No supported storage found'}
          </p>
          {!volumes.length && !inventoryIncomplete && (
            <p className="text-xs leading-relaxed text-muted-foreground">{emptyStorageMessage}</p>
          )}
          {unknownCount > 0 && (
            <p className="text-xs leading-relaxed text-muted-foreground">
              Coverage unavailable for {unknownCount} {unknownCount === 1 ? 'volume' : 'volumes'}.
            </p>
          )}
          {lastSuccessful && (
            <p className="text-xs leading-relaxed text-muted-foreground">
              Most recent successful backup: {new Date(lastSuccessful).toLocaleString()}
            </p>
          )}
        </div>
        {isWorkload &&
          coverage.canConfigure &&
          volumes.length > 0 &&
          excludedVolumes.length === 0 &&
          !existingWorkloadPolicy &&
          policies.length === 0 &&
          unknownCount === 0 && (
            <Button asChild variant="outline" size="sm">
              <Link to={backupConfigurationUrl(source, returnTo)}>Configure backup</Link>
            </Button>
          )}
      </div>
      {rows.length > 0 && (
        <ul className="divide-y rounded-lg border px-3">
          {rows.map(({ volume, coverage: item }) => (
            <li key={JSON.stringify([volume.dockerNodeId ?? null, volume.name])} className="space-y-2 py-3">
              <div className="flex min-w-0 items-start gap-2">
                {volume.hostPath ? (
                  <FolderOpen className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
                ) : (
                  <Database className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
                )}
                <div className="min-w-0 flex-1">
                  {volume.hostPath ? (
                    <span className="block truncate text-sm font-medium" title={volume.hostPath}>
                      {volume.hostPath}
                    </span>
                  ) : (
                    <Link
                      className="table-link block truncate text-sm font-medium"
                      title={volume.name}
                      to={volumeUrl(coverage.platformId, volume)}>
                      {volume.name}
                    </Link>
                  )}
                  <div className="mt-1 flex flex-wrap gap-x-2 gap-y-1 text-xs text-muted-foreground">
                    <span>{volume.hostPath ? 'Bind mount' : 'Docker volume'}</span>
                    {volume.dockerNodeId && (
                      <span className="break-all">{volume.nodeHostname ?? volume.dockerNodeId}</span>
                    )}
                  </div>
                </div>
                <CoverageIndicator
                  volumeName={volume.name}
                  kind={volume.hostPath ? 'bind mount' : 'volume'}
                  coverage={item}
                  label
                />
              </div>
              {source.$type === 'Deployment' && !volume.hostPath && !volume.includedInResourcePolicy && (
                <p className="text-xs text-muted-foreground">Not included in deployment backups</p>
              )}
              {(volume.hostPath ? coverage.canConfigureBindMount : coverage.canConfigure) &&
                item.status === BackupCoverageStatus.Unprotected &&
                (!existingWorkloadPolicy || !volume.includedInResourcePolicy) && (
                  <Link
                    className="inline-flex text-xs font-medium text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
                    to={backupConfigurationUrl(
                      volume.hostPath
                        ? {
                            $type: 'BindMount',
                            platformId: coverage.platformId,
                            hostPath: volume.hostPath,
                            dockerNodeId: volume.dockerNodeId,
                          }
                        : {
                            $type: 'DockerVolume',
                            platformId: coverage.platformId,
                            volumeName: volume.name,
                            dockerNodeId: volume.dockerNodeId,
                          },
                      returnTo,
                    )}
                    aria-label={`Configure backup for ${volume.name}`}>
                    Configure backup
                  </Link>
                )}
            </li>
          ))}
        </ul>
      )}
      {backupConfigurationIssue(policies) && (
        <div className="space-y-2 rounded-lg bg-amber-500/5 p-3 text-xs leading-relaxed text-amber-700 dark:text-amber-400">
          <p className="font-medium">{backupConfigurationIssue(policies)}</p>
          <p>
            {lastSuccessful
              ? 'A successful backup exists, but ongoing protection needs attention.'
              : 'Enable a schedule and complete a successful backup for ongoing protection.'}
          </p>
          {policies
            .filter((p) => p.scheduleConfigured === false && p.canWrite)
            .map((p) => (
              <Link
                key={p.id}
                className="block underline underline-offset-4"
                to={`/backup-policies/edit/${p.id}?${new URLSearchParams({ returnTo, section: 'schedule' })}#config`}>
                Edit schedule{policies.length > 1 ? `: ${p.name}` : ''}
              </Link>
            ))}
        </div>
      )}
      {policies.length > 0 && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-t pt-3 text-xs">
          <span className="text-muted-foreground">Manage {policies.length === 1 ? 'policy' : 'policies'}:</span>
          {policies.map((policy) => (
            <Link
              key={policy.id}
              className="underline underline-offset-4"
              to={`/backup-policies/edit/${policy.id}?${new URLSearchParams({ returnTo })}`}>
              {policy.name}
            </Link>
          ))}
        </div>
      )}
      {backupCoverageMessages(coverage).map((warning) => (
        <div
          key={warning}
          className="flex items-start gap-2 rounded-lg bg-amber-500/5 p-3 text-xs leading-relaxed text-amber-700 dark:text-amber-400">
          <TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
          <p>{warning}</p>
        </div>
      ))}
      {source.$type === 'Deployment' && excludedVolumes.some((volume) => !volume.hostPath) && (
        <p className="text-xs text-muted-foreground">
          Deployment backups include declared named volumes and supported bind mounts on standalone Docker. Other Docker
          volumes need a separate volume policy.
        </p>
      )}
      <details className="group border-t pt-3 text-xs text-muted-foreground">
        <summary className="flex cursor-pointer list-none items-center justify-between gap-2 rounded-sm font-medium transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
          What’s included
          <ChevronDown
            className="size-3.5 transition-transform group-open:rotate-180 motion-reduce:transition-none"
            aria-hidden="true"
          />
        </summary>
        <p className="pt-2 leading-relaxed">
          {source.$type === 'Container'
            ? 'Back up each eligible Docker volume or bind mount through a storage or workload policy. Tmpfs and container writable layers are excluded.'
            : source.$type === 'DockerVolume'
              ? 'This policy backs up the selected Docker volume, including volumes with Docker-generated names.'
              : source.$type === 'BindMount'
                ? 'This policy backs up the selected host path on its source node. Unsupported host paths and nested mounts are excluded.'
                : 'Workload backups include eligible Docker volumes and supported bind mounts. Tmpfs, nested mounts, and container writable layers are excluded.'}
          {volumes.length > 0 && ' Coverage reflects policies you can access. Manage policies in Backups.'}
        </p>
      </details>
    </div>
  );
}

function volumeUrl(platformId: string, volume: VolumeTarget) {
  return `/platforms/${platformId}/volumes/${encodeURIComponent(volume.name)}${volume.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(volume.dockerNodeId)}` : ''}`;
}
