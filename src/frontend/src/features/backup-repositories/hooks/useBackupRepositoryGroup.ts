import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { BackupRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useBackupRepositoryGroup(id: string) {
  const queryClient = useQueryClient();
  const query = useRead('getBackupRepository', { id });
  const receive = useCallback(
    (repository: BackupRepositoryView, operation: string) => {
      if (repository.id !== id || operation === 'delete') return;
      const filter = { queryKey: ['getBackupRepository', { id }], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData(filter.queryKey, { data: repository });
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('BackupRepositoryInfoUpdated', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('BackupRepositoryInfoUpdated', receive),
    [receive],
  );
  useRealtimeGroup({ groupName: `backup-repository:${id}`, setupEventListeners, removeEventListeners });
  return query;
}
