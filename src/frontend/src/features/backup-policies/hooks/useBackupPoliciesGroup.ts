import { BackupPolicyView, ResourceCapabilitiesView } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

export const useBackupPoliciesGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listBackupPolicies', readArgs);
  const [policies, setPolicies] = useState<BackupPolicyView[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilitiesView | undefined>();
  const lastFetchedRef = useRef<BackupPolicyView[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.policies;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setPolicies(newBase);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (policy: BackupPolicyView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((policy.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const handleBackupPolicyInfoUpdated = useCallback(
    (policy: BackupPolicyView, action: string) => {
      setPolicies((prev) => {
        if (!prev) return prev;
        if (action === 'create') {
          return matchesActiveFilters(policy) ? [...prev, policy] : prev;
        }
        if (action === 'delete') {
          return prev.filter((item) => item.id !== policy.id);
        }

        const index = prev.findIndex((item) => item.id === policy.id);
        if (!matchesActiveFilters(policy)) {
          return index === -1 ? prev : prev.filter((item) => item.id !== policy.id);
        }

        if (index === -1) return [...prev, policy];

        const updated = [...prev];
        updated[index] = {
          ...policy,
          latestRun: policy.latestRun ?? updated[index].latestRun,
        };
        return updated;
      });
    },
    [matchesActiveFilters],
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
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, policies: policies ?? [], isLoading, capabilities, selectedTagNames };
};
