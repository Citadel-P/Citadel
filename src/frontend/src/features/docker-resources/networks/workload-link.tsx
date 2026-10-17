import { Link } from 'react-router';
import type { NetworkUsageEntry } from '@/api/generated/api.types';
import { StateIndicator } from '@/components/custom/state-indicator';
type WorkloadIdentity = Pick<
  NetworkUsageEntry,
  'workloadId' | 'workloadName' | 'workloadKind' | 'workloadStatus' | 'workloadProcessing'
>;
export function WorkloadLink({ entry }: { entry: WorkloadIdentity }) {
  if (!entry.workloadId || !entry.workloadName) return null;
  const base =
    entry.workloadKind === 'Deployment' ? 'deployments' : entry.workloadKind === 'Stack' ? 'stacks' : 'swarm-services';
  return (
    <div className="flex min-w-0 items-center gap-2">
      <StateIndicator
        value={entry.workloadStatus ?? 'Unknown'}
        isProcessing={entry.workloadProcessing}
        className="mr-0"
      />
      <Link className="table-link break-all text-xs" to={`/${base}/edit/${entry.workloadId}/#config`}>
        {entry.workloadName}
      </Link>
    </div>
  );
}
