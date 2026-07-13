import { BackupPolicyView } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';

export const useBackupPoliciesGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading } = useRead('listBackupPolicies', readArgs);

  return {
    policies: (data?.data.policies ?? []) as BackupPolicyView[],
    isLoading,
    capabilities: data?.data.capabilities,
    selectedTagNames,
  };
};
