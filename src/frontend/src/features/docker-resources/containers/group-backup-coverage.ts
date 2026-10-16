import {
  BackupCoverageStatus as Status,
  type BackupPolicyReference,
  type ContainerView,
  type ResourceBackupSummaryView,
  type ResourceBackupVolumeView,
} from '@/api/generated/api.types';

// Summarize already-authorized observations, not a saved Stack definition.
// Keep the same status precedence as the server's coverage::aggregate.
function aggregate(statuses: (Status | null | undefined)[]): Status | null {
  const applicable = statuses.filter((status) => status !== Status.NotApplicable);
  if (applicable.includes(Status.Failed)) return Status.Failed;
  if (applicable.includes(Status.Warning)) return Status.Warning;
  if (applicable.some((status) => status == null)) return null;
  if (!applicable.length) return Status.NotApplicable;
  if (applicable.every((status) => status === Status.Protected)) return Status.Protected;
  if (applicable.every((status) => status === Status.Unprotected)) return Status.Unprotected;
  return Status.Warning;
}

function mergePolicies(policies: BackupPolicyReference[]): BackupPolicyReference[] {
  const byId = new Map<string, BackupPolicyReference>();
  for (const policy of policies) {
    const previous = byId.get(policy.id);
    byId.set(
      policy.id,
      previous
        ? {
            ...policy,
            canWrite: Boolean(previous.canWrite && policy.canWrite),
            enabled: previous.enabled === policy.enabled ? policy.enabled : null,
            scheduleConfigured:
              previous.scheduleConfigured === policy.scheduleConfigured ? policy.scheduleConfigured : null,
          }
        : { ...policy },
    );
  }
  return [...byId.values()];
}

export function groupBackupCoverage(containers: ContainerView[]): ResourceBackupSummaryView | undefined {
  const platformId = containers[0]?.platformId;
  if (!platformId) return undefined;
  const summaries: ResourceBackupSummaryView[] = [];
  for (const container of containers) {
    const summary = container.backupCoverage;
    if (
      container.projectionStaleSince ||
      container.platformId !== platformId ||
      !summary ||
      summary.unavailable ||
      summary.platformId !== platformId
    )
      return undefined;
    summaries.push(summary);
  }
  if (!summaries.length) return undefined;

  const targets = new Map<string, ResourceBackupVolumeView>();
  for (const summary of summaries) {
    for (const volume of summary.volumes) {
      const key = JSON.stringify([
        platformId,
        volume.dockerNodeId ?? null,
        volume.hostPath ? 'bind' : 'volume',
        volume.hostPath ?? volume.name,
      ]);
      const previous = targets.get(key);
      targets.set(key, {
        ...volume,
        // An unmanaged group has no workload policy of its own.
        includedInResourcePolicy: false,
        status: previous ? aggregate([previous.status, volume.status]) : volume.status,
        policies: mergePolicies([...(previous?.policies ?? []), ...volume.policies]),
        lastSuccessfulRunAt: [previous?.lastSuccessfulRunAt, volume.lastSuccessfulRunAt]
          .filter((value): value is string => Boolean(value))
          .sort()
          .at(-1),
      });
    }
  }
  const volumes = [...targets.values()];
  return {
    platformId,
    status: aggregate([...summaries.map((summary) => summary.status), ...volumes.map((volume) => volume.status)]),
    volumes,
    policies: mergePolicies(summaries.flatMap((summary) => summary.policies)),
    hasResourcePolicy: false,
    canConfigure: summaries.every((summary) => summary.canConfigure),
    canConfigureBindMount: summaries.every((summary) => summary.canConfigureBindMount === true),
    unavailable: false,
    warnings: [...new Set(summaries.flatMap((summary) => summary.warnings))],
    reasons: [...new Set(summaries.flatMap((summary) => summary.reasons))],
  };
}
