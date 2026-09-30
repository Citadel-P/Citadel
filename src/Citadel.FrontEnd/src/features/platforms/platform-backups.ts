import { PlatformBackupSummary } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { WorkloadState } from './platform-workload-status';

export const PLATFORM_BACKUP_ICON_CLASS_NAME = 'text-teal-500';

export const usePlatformBackupSummaries = (platformIds: string[]) => {
  const platformIdsKey = platformIds.join(',');
  const requestedIds = useMemo(
    () => [...new Set(platformIds.filter(Boolean))].sort(),
    // The joined value keeps the query stable when callers recreate the same ID array.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [platformIdsKey],
  );
  const readArgs = useMemo(() => ({ query: { platformIds: requestedIds } }), [requestedIds]);
  const query = useRead('getPlatformBackupSummaries', readArgs, { enabled: requestedIds.length > 0 });
  const summaries = useMemo(
    () => new Map(query.data?.data.platforms.map((summary) => [summary.platformId, summary]) ?? []),
    [query.data?.data.platforms],
  );

  return { summaries, isLoading: query.isLoading, isError: query.isError };
};

export const getBackupSourceStates = (summary?: PlatformBackupSummary): WorkloadState[] => [
  {
    label: 'Volume policies',
    value: summary?.dockerVolumePolicyCount,
    colorClassName: 'bg-amber-500',
  },
  {
    label: 'Stack policies',
    value: summary?.stackPolicyCount,
    colorClassName: 'bg-violet-500',
  },
  {
    label: 'Deployment policies',
    value: summary?.deploymentPolicyCount,
    colorClassName: 'bg-sky-500',
  },
];

export const getBackupMetric = (summary: PlatformBackupSummary | undefined, isLoading: boolean, isError = false) => {
  if (isLoading) {
    return { value: '-', detail: 'Loading', detailTitle: 'Loading backup policies' };
  }

  if (isError || !summary) {
    return { value: '-', detail: 'Unavailable', detailTitle: 'Backup policy data is unavailable' };
  }

  const policyCount = Number(summary.policyCount);
  const enabledPolicyCount = Number(summary.enabledPolicyCount);
  const attentionPolicyCount = Number(summary.attentionPolicyCount);
  const detail =
    policyCount === 0
      ? 'No policies'
      : attentionPolicyCount > 0
        ? `${attentionPolicyCount} ${attentionPolicyCount === 1 ? 'needs' : 'need'} attention`
        : `${enabledPolicyCount} active`;

  return {
    value: policyCount,
    detail,
    detailTitle: [
      `${Number(summary.dockerVolumePolicyCount)} volume`,
      `${Number(summary.stackPolicyCount)} stack`,
      `${Number(summary.deploymentPolicyCount)} deployment`,
    ].join(', '),
  };
};
