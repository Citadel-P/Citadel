import type { JobStatusView } from '@/api/generated/api.types';

// Explanations describe the measured worker boundary, not business-operation success.
const descriptions: Record<string, [string, string]> = {
  'resource-scheduler': ['Schedule resource work', 'Tracks deadlines and asks each resource to accept work when it is due.'],
  'container-stats-writer': [
    'Save container measurements',
    'Saves batches of container measurements. Batch sizes vary; elapsed time includes database waits.',
  ],
  'platform-stats-writer': [
    'Save platform measurements',
    'Saves batches of platform measurements. Batch sizes vary; elapsed time includes database waits.',
  ],
  'history-retention': [
    'Remove expired history',
    'Prunes expired monitoring and operation history according to retention settings.',
  ],
  'operation-recovery': [
    'Reconcile unfinished operations',
    'Checks persisted operation state and reconciles work needing recovery.',
  ],
  'lease-expiry': [
    'Check expired reservations',
    'Checks expiring execution reservations using existing recovery rules.',
  ],
  'backup-coverage-realtime': ['Update backup coverage views', 'Publishes backup coverage changes to connected pages.'],
  'resource-copy-realtime': ['Update copy progress views', 'Publishes resource-copy changes to connected pages.'],
  'resource-copies': [
    'Coordinate resource copies',
    'Coordinates queued resource copies. Individual operations hold their own results.',
  ],
  'alert-configuration-cache': ['Keep alert settings current', 'Updates the alert configuration used by workers.'],
  'platform-targets': [
    'Keep platform connections current',
    'Refreshes the runtime targets used to communicate with platforms.',
  ],
  'job-alert-observations': [
    'Evaluate operation alerts',
    'Processes pending operation observations for alert evaluation.',
  ],
  'volume-helper-recovery': ['Reconcile volume helpers', 'Checks volume-helper state and recovery work.'],
  'container-operation-reconciliation': [
    'Reconcile container operations',
    'Reconciles container operations with observed runtime state.',
  ],
  'stack-event-drift': [
    'Check stack changes after events',
    'Uses runtime events to check for differences from stack configuration.',
  ],
  'git-repository-sync': ['Synchronize Git repositories', 'Processes queued repository synchronization work.'],
  'scripts-cleanup': ['Clean up script execution resources', 'Processes cleanup associated with script execution.'],
  'platform-threshold-alerts': [
    'Check monitoring alert thresholds',
    'Evaluates monitoring observations against configured alert rules.',
  ],
  'alert-deliveries': [
    'Process notification deliveries',
    'Processes queued alert delivery work. A worker result is not confirmation that every recipient received a notification.',
  ],
  'build-consumers': [
    'Process build execution work',
    'Coordinates build execution consumers. Individual build runs contain their results.',
  ],
  'build-cleanup': ['Clean up build resources', 'Processes cleanup associated with build execution.'],
  'build-pool-health': [
    'Check build pool availability',
    'Checks the availability of configured build execution targets.',
  ],
  'backup-cleanup': ['Reconcile backup cleanup', 'Checks cleanup reservations for backup and restore work.'],
  'deployment-apply-reconciliation': [
    'Reconcile deployment operations',
    'Reconciles deployment operations with their observed state.',
  ],
  'deployment-image-updates': ['Check deployment image updates', 'Checks image update work for deployments.'],
  'swarm-service-operation-reconciliation': [
    'Reconcile Swarm service operations',
    'Reconciles service operations with Swarm observations.',
  ],
  'swarm-service-active-operations': [
    'Track active Swarm service work',
    'Processes observations for active service operations.',
  ],
  'swarm-service-image-updates': [
    'Check Swarm service image updates',
    'Checks image update work for managed Swarm services.',
  ],
  'stack-operation-reconciliation': [
    'Reconcile stack operations',
    'Reconciles managed stack operations with runtime observations.',
  ],
  'stack-drift-monitor': [
    'Check for stack configuration drift',
    'Checks whether observed stacks differ from their managed configuration.',
  ],
  'stack-webhooks': ['Process stack webhook work', 'Processes stack work queued by webhook requests.'],
  'stack-updates': ['Check stack updates', 'Processes update checks for managed stacks.'],
  'unmanaged-container-alerts': [
    'Check unmanaged container alerts',
    'Checks unmanaged-container observations against applicable alert rules.',
  ],
  'platform-lifecycle-coordinator': [
    'Coordinate platform state changes',
    'Processes platform lifecycle changes and reconnect work.',
  ],
  'docker-event-source': [
    'Listen for Docker changes',
    'Receives Docker lifecycle events. Waiting for an event is normal.',
  ],
  'agent-subscriptions': ['Listen for Agent observations', 'Receives observations from connected Agents.'],
  'event-resource-refresh': [
    'Refresh resources changed by events',
    'Updates resource observations after relevant runtime events.',
  ],
  'docker-event-consumer': ['Process Docker changes', 'Processes received Docker lifecycle events.'],
  'update-installer': [
    'Coordinate Citadel installation updates',
    'Coordinates approved installation update work. The Updates page contains its operation state.',
  ],
  'service-account-last-used': [
    'Save service-account usage times',
    'Batches last-used timestamps for service accounts.',
  ],
  'license-transition-monitor': [
    'Observe license changes',
    'Observes license transitions and publishes the resulting state changes.',
  ],
  'edge-platform-inventory': [
    'Receive Edge platform inventory',
    'Processes inventory observations from Edge sessions.',
  ],
  'core-job-lease': [
    'Maintain background-work ownership',
    'Checks that this Core still holds the database lease required to run background work.',
  ],
  'automatic-update-setup': [
    'Prepare automatic update support',
    'Prepares installation update support where available.',
  ],

  'statistics-compaction': [
    'Optimize older monitoring history',
    'Combines older measurements into summaries. A finished batch does not mean all remaining history has been processed.',
  ],
  'statistics-retention': [
    'Clean up expired monitoring history',
    'Removes monitoring data outside its retention period in bounded batches.',
  ],
  'stats-retention': [
    'Clean up expired monitoring history',
    'Removes monitoring data outside its retention period in bounded batches.',
  ],
  'backup-policy-scheduler': [
    'Check backup schedules',
    'Checks which backups are due and queues work. This does not confirm that a backup succeeded.',
  ],
  'backup-runs': [
    'Dispatch backup work',
    'Claims queued backup operations. Timings describe queue claims; backup results are recorded in Backups.',
  ],
  'backup-restore-runs': [
    'Dispatch restore work',
    'Claims queued restore operations. Timings describe queue claims; restore results are recorded with the operation.',
  ],
  'build-runs': [
    'Dispatch build work',
    'Finds and claims queued builds. Check build runs for their execution results.',
  ],
  'scripts-scheduler': [
    'Check script schedules',
    'Queues scripts when their schedules are due. Script runs report execution results separately.',
  ],
  'scripts-runs': ['Dispatch script work', 'Processes queued script work. Check script runs for individual results.'],
  'local-container-stats': [
    'Collect container measurements',
    'Collects monitoring samples from local containers. Saving those samples is measured separately.',
  ],
  'platform-health-monitor': [
    'Check platform connectivity',
    'Observes platform connectivity and reports changes. This is not a health verdict for every application.',
  ],
  'platform-inventory-reconciliation': [
    'Keep platform inventory up to date',
    'Reconciles observed Docker resources with Citadel’s inventory.',
  ],
  'platform-deployment-catch-up': [
    'Synchronize deployment observations',
    'Catches up deployment observations after connectivity or inventory changes.',
  ],
  'swarm-task-pruning': ['Clean up old Swarm tasks', 'Processes eligible Swarm task cleanup.'],
  'update-checker': ['Check for Citadel updates', 'Checks for available releases. It does not install an update.'],
  'agent-updates': ['Coordinate Agent updates', 'Checks and coordinates Agent update work.'],
  'image-scanner': ['Scan container images', 'Processes requested image scans and records their results.'],
  'inventory-image-updates': ['Check for newer images', 'Checks image registries for available updates.'],
  'database-notifications': [
    'Receive database changes',
    'Receives database notifications that wake other workers. Waiting for a change is normal.',
  ],
  'connection-realtime-updates': [
    'Keep connected pages up to date',
    'Delivers changes to connected clients. Waiting for changes is normal.',
  ],
  'job-history-maintenance': [
    'Optimize older job history',
    'Combines older work measurements into hourly summaries and removes expired job diagnostics.',
  ],
  'job-diagnostics': [
    'Save background-work diagnostics',
    'Saves bounded diagnostic history. Execution continues if diagnostic storage is unavailable.',
  ],
};
export function describeJob(job: Pick<JobStatusView, 'id' | 'name'>) {
  const [name, purpose] = descriptions[job.id] ?? [
    job.name,
    'Background work reported by this Core. A detailed explanation is not available for this job yet.',
  ];
  return { name, purpose };
}
export function latestResult(job: JobStatusView) {
  const count = job.lastCompletedUnits;
  const measured = count != null && job.lastUnit != null;
  if (job.id === 'local-container-stats' && job.lastUnit === 'samples' && count != null) {
    if (job.lastOutcome === 'failed' && job.lastFailedUnits != null) {
      return `Collected ${count.toLocaleString()} samples; ${job.lastFailedUnits.toLocaleString()} containers could not be measured`;
    }
    if (job.lastOutcome === 'productive') {
      return `Collected measurements from ${count.toLocaleString()} ${count === 1 ? 'container' : 'containers'}`;
    }
    if (job.lastOutcome === 'noWork') return 'No container measurements to collect';
  }
  switch (job.lastOutcome) {
    case 'noWork':
      return 'Nothing to process in the last check';
    case 'productive':
      if (measured && job.lastUnit === 'claims')
        return `Claimed ${count.toLocaleString()} queued ${count === 1 ? 'operation' : 'operations'}`;
      if (measured && (job.id === 'container-stats-writer' || job.id === 'platform-stats-writer'))
        return `Saved ${count.toLocaleString()} ${job.lastUnit}`;
      return measured ? `Processed ${count.toLocaleString()} ${job.lastUnit}` : 'Last check processed work';
    case 'completed':
      return 'Last check finished · work count unavailable';
    case 'failed':
      return 'Last attempt failed';
    case 'interrupted':
      return 'Work was interrupted';
    case 'cancelled':
      return 'Work was cancelled';
    case 'budgetExhausted':
      return 'Batch time limit reached · work may remain';
    default:
      return 'No measured result yet';
  }
}
export function attention(job: JobStatusView) {
  if (job.state === 'unavailable')
    return job.lastOutcome === 'failed' || job.lastOutcome === 'interrupted' ? 'attention' : 'unknown';
  if (job.state === 'backingOff') return job.nextCheck != null ? 'retrying' : 'attention';
  if (
    (job.backlog ?? 0) > 0 &&
    job.state === 'running' &&
    job.lastProgressAt != null &&
    job.currentStartedAt != null &&
    job.lastProgressAt >= job.currentStartedAt
  )
    return 'catchingUp';
  return undefined;
}
