import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { BackupPolicyView } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo } from 'react';

const mergeRun = (incoming: BackupPolicyView, existing: BackupPolicyView) => ({
  ...incoming,
  latestRun: incoming.latestRun ?? existing.latestRun,
});

export const useBackupPoliciesGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = collectionGroup('backup-policies', selectedTagNames);
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listBackupPolicies', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.policies : undefined;
  }, [refetch]);
  const [policies, handleBackupPolicyInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.policies,
    JSON.stringify(readArgs ?? {}),
    reload,
    mergeRun,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BackupPolicyInfoUpdated', handleBackupPolicyInfoUpdated);
    },
    [handleBackupPolicyInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BackupPolicyInfoUpdated', handleBackupPolicyInfoUpdated);
    },
    [handleBackupPolicyInfoUpdated],
  );

  useRealtimeGroup({
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, policies: policies ?? [], isLoading, capabilities, selectedTagNames };
};
