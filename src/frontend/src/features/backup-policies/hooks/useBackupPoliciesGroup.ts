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
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listBackupPolicies', readArgs);
  const capabilities = data?.data.capabilities;
  const matchesActiveFilters = useCallback(
    (policy: BackupPolicyView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((policy.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.policies : undefined;
  }, [refetch]);
  const [policies, handleBackupPolicyInfoUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.policies,
    JSON.stringify(readArgs ?? {}),
    matchesActiveFilters,
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
    groupName: 'backup-policies',
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, policies: policies ?? [], isLoading, capabilities, selectedTagNames };
};
