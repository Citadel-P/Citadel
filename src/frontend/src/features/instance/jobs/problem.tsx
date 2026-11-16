import { Link } from 'react-router';
import { ActivityResourceType, ResourceType, type JobTargetView } from '@/api/generated/api.types';
import { TargetCell } from '@/components/custom/common';

// Reuse the same managed-resource destinations as Activities.
const resourceTargets: Partial<Record<ResourceType, ActivityResourceType>> = {
  [ResourceType.Platform]: ActivityResourceType.Platform,
  [ResourceType.Deployment]: ActivityResourceType.Deployment,
  [ResourceType.Stack]: ActivityResourceType.Stack,
  [ResourceType.SwarmService]: ActivityResourceType.SwarmService,
  [ResourceType.Registry]: ActivityResourceType.Registry,
  [ResourceType.GitRepository]: ActivityResourceType.GitRepository,
  [ResourceType.Script]: ActivityResourceType.Script,
  [ResourceType.Build]: ActivityResourceType.Build,
  [ResourceType.BuildAgentPool]: ActivityResourceType.BuildAgentPool,
  [ResourceType.BackupPolicy]: ActivityResourceType.BackupPolicy,
  [ResourceType.User]: ActivityResourceType.User,
  [ResourceType.Team]: ActivityResourceType.Team,
  [ResourceType.ServiceAccount]: ActivityResourceType.ServiceAccount,
};

// Only link routes that can identify the reported target without another lookup.
// Destination pages retain their normal authorization and missing-resource handling.
export function JobProblemTarget({ target }: { target?: JobTargetView | null }) {
  if (!target) return null;
  if (target.resourceType === ResourceType.Platform && target.containerId) {
    return (
      <Link
        className="table-link"
        to={`/platforms/${encodeURIComponent(target.resourceId)}/containers/${encodeURIComponent(target.containerId)}`}>
        Open affected container
      </Link>
    );
  }
  if (target.resourceType === ResourceType.Build && target.operationId) {
    return (
      <Link
        className="table-link"
        to={`/builds/edit/${encodeURIComponent(target.resourceId)}?runId=${encodeURIComponent(target.operationId)}#runs`}>
        Open build run
      </Link>
    );
  }
  const activityType = resourceTargets[target.resourceType];
  if (activityType) {
    const label = target.resourceType.replace(/([a-z])([A-Z])/g, '$1 $2').toLowerCase();
    return (
      <TargetCell resourceType={activityType} resourceId={target.resourceId} resourceName={`Open affected ${label}`} />
    );
  }
  return (
    <span className="text-muted-foreground">
      {target.resourceType} · {target.resourceId}
    </span>
  );
}
