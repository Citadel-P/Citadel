import {
  BackupCoverageStatus,
  BackupPolicyReference,
  BackupSourceSpec,
  BackupVolumePreview,
  ResourceBackupSummaryView,
} from '@/api/generated/api.types';

export type VolumeTarget = { name: string; dockerNodeId?: string | null; nodeHostname?: string | null };
export type Coverage = {
  status?: BackupCoverageStatus;
  policies: BackupPolicyReference[];
  lastSuccessfulRunAt?: string | null;
};

export function sourceKey(source: BackupSourceSpec): string {
  switch (source.$type) {
    case 'BindMount':
      return JSON.stringify([source.$type, source.platformId, source.dockerNodeId ?? null, source.hostPath]);
    case 'DockerVolume':
      return JSON.stringify([source.$type, source.platformId, source.dockerNodeId ?? null, source.volumeName]);
    case 'Stack':
      return `Stack:${source.stackId}`;
    case 'Deployment':
      return `Deployment:${source.deploymentId}`;
    case 'SwarmService':
      return `SwarmService:${source.swarmServiceId}`;
    default:
      return 'CitadelSystem';
  }
}

export const sameVolume = (left: VolumeTarget, right: VolumeTarget) =>
  left.name === right.name && (left.dockerNodeId ?? null) === (right.dockerNodeId ?? null);

export function isWorkloadBackupVolume(sourceType: BackupSourceSpec['$type'], volume: BackupVolumePreview): boolean {
  if (volume.kind === 'BindMount')
    return (
      sourceType === 'Stack' || sourceType === 'SwarmService' || (sourceType === 'Deployment' && !volume.dockerNodeId)
    );
  // Deployment previews also contain runtime-only mounts. Execution uses only
  // named sources in the saved definition and does not support Swarm deployments.
  if (sourceType === 'Deployment') return volume.kind === 'DeclaredNamed' && !volume.dockerNodeId;
  // Stack bindings include anonymous volumes; Swarm previews use the execution planner.
  return sourceType === 'Stack' || sourceType === 'SwarmService';
}

export function backupConfigurationIssue(policies: BackupPolicyReference[]) {
  if (!policies.length) return undefined;
  if (policies.every((p) => p.enabled === false)) return 'Backup policies disabled';
  if (policies.every((p) => p.scheduleConfigured === false)) return 'Automatic backups not configured';
  if (policies.some((p) => p.scheduleConfigured === false)) return 'Some policies have no schedule';
  return undefined;
}

export function backupSummaryStatus(coverage?: ResourceBackupSummaryView | null): BackupCoverageStatus | 'Unavailable' {
  if (!coverage || coverage.unavailable) return 'Unavailable';
  return coverage.status ?? 'Unavailable';
}

export function backupCoverageMessages(coverage: ResourceBackupSummaryView): string[] {
  const messages = (coverage.reasons ?? [])
    .map((reason) => {
      switch (reason) {
        case 'InventoryIncomplete':
          return 'Storage inventory is incomplete. Apply the workload and synchronize its inventory to confirm coverage.';
        case 'InventoryUnavailable':
          return 'Storage inventory is unavailable. Synchronize the platform to confirm coverage.';
        case 'SchedulingUnavailable':
          return 'Automatic backups are paused. Scheduling requires Automated Operations and enabled backup workers.';
      }
    })
    .filter((message) => message !== undefined);
  return [...new Set([...messages, ...coverage.warnings])];
}
