import { SwarmNodesView } from '@/api/generated/api.types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

type CachedResponse<T> = { data: T };

export const useSwarmNodes = (platformId?: string) => {
  const query = useRead('listSwarmNodes', { platformId });
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (updated: { platformId: string; nodes: SwarmNodesView }) => {
      if (updated.platformId !== platformId) return;
      const queryKey = ['listSwarmNodes', { platformId }] as const;
      queryClient.setQueryData<CachedResponse<SwarmNodesView>>(queryKey, (previous) =>
        previous ? { ...previous, data: updated.nodes } : { data: updated.nodes },
      );
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
    },
    [platformId, queryClient],
  );
  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });

  return { nodes: query.data?.data, isLoading: query.isLoading, error: query.error };
};
