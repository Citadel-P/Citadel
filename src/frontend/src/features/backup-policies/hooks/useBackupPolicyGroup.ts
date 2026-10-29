import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { BackupPolicyView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useBackupPolicyGroup(id: string) {
  const queryClient = useQueryClient();
  const query = useRead('getBackupPolicy', { id });
  const receive = useCallback(
    (policy: BackupPolicyView, operation: string) => {
      if (policy.id !== id || operation === 'delete') return;
      const filter = { queryKey: ['getBackupPolicy', { id }], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData(filter.queryKey, { data: policy });
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('BackupPolicyInfoUpdated', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('BackupPolicyInfoUpdated', receive),
    [receive],
  );
  useRealtimeGroup({ groupName: `backup-policy:${id}`, setupEventListeners, removeEventListeners });
  return query;
}
