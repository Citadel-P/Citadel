import { BackupSourceSpec, VolumeBackupConsistency } from '@/api/generated/api.types';

export function backupConfigurationUrl(source: BackupSourceSpec, returnTo: string) {
  const query = new URLSearchParams({ sourceType: source.$type, returnTo });
  for (const [key, value] of Object.entries(source)) {
    if (key !== '$type' && value != null) query.set(key, String(value));
  }
  return `/backup-policies/add?${query}`;
}

export function backupSourceFromSearch(search: URLSearchParams): BackupSourceSpec | undefined {
  switch (search.get('sourceType')) {
    case 'BindMount': {
      const platformId = search.get('platformId');
      const hostPath = search.get('hostPath');
      return platformId && hostPath
        ? { $type: 'BindMount', platformId, hostPath, dockerNodeId: search.get('dockerNodeId') }
        : undefined;
    }
    case 'DockerVolume': {
      const platformId = search.get('platformId');
      const volumeName = search.get('volumeName');
      if (!platformId || !volumeName) return undefined;
      return {
        $type: 'DockerVolume',
        platformId,
        volumeName,
        dockerNodeId: search.get('dockerNodeId'),
        consistency: VolumeBackupConsistency.Live,
      };
    }
    case 'Stack':
      return search.get('stackId') ? { $type: 'Stack', stackId: search.get('stackId')! } : undefined;
    case 'SwarmService':
      return search.get('swarmServiceId')
        ? { $type: 'SwarmService', swarmServiceId: search.get('swarmServiceId')! }
        : undefined;
    case 'Deployment':
      return search.get('deploymentId')
        ? { $type: 'Deployment', deploymentId: search.get('deploymentId')! }
        : undefined;
    default:
      return undefined;
  }
}

export function backupReturnPath(search: URLSearchParams) {
  const path = search.get('returnTo');
  // Only return to local resource pages, never an arbitrary URL supplied in a query.
  return path &&
    !path.includes('\\') &&
    !/[\r\n]/.test(path) &&
    /^\/(?:deployments\/edit\/[^/?#]+|stacks\/edit\/[^/?#]+|swarm-services\/edit\/[^/?#]+|platforms\/[^/?#]+\/(?:containers(?:\/[^/?#]+)?|volumes(?:\/[^/?#]+)?))\/?(?:[?#].*)?$/.test(
      path,
    )
    ? path
    : undefined;
}
