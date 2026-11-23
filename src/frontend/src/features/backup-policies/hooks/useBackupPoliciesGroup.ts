import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { BackupPolicyView } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';

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
  const query = useRead('listBackupPolicies', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;

  const policies = useCollectionSubscription({
    groupName,
    eventName: 'BackupPolicyInfoUpdated',
    enabled: filterReady,
    field: 'policies',
    query,
    merge: mergeRun,
  });
  return { error, refetch, isFetching, policies: policies ?? [], isLoading, capabilities, selectedTagNames };
};
