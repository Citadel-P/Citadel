import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ResourceCopyProgress } from '@/api/generated/api.types';
import type { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useResourceCopyGroup(id: string, enabled: boolean) {
  const queryClient = useQueryClient();
  const operation = useRead('getResourceCopy', { id }, { enabled: enabled && Boolean(id) });
  const onProgress = useCallback(
    (progress: ResourceCopyProgress) => {
      if (progress.id !== id) return;
      const queryKey = ['getResourceCopy', { id }] as const;
      // A slow initial HTTP read must not replace a newer realtime snapshot.
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<ResourceResponse<'getResourceCopy'>>(
        queryKey,
        (current) =>
          ({
            ...current,
            data: progress,
          }) as ResourceResponse<'getResourceCopy'>,
      );
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('ResourceCopyInfoUpdated', onProgress),
    [onProgress],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('ResourceCopyInfoUpdated', onProgress),
    [onProgress],
  );
  useRealtimeGroup({
    groupName: id ? `resource-copy:${id}` : undefined,
    skip: !enabled || !id,
    setupEventListeners,
    removeEventListeners,
  });
  return operation;
}
