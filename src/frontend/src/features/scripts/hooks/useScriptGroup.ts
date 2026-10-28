import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { AuthorizedScript } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useScriptGroup(id: string) {
  const queryClient = useQueryClient();
  const query = useRead('getScript', { id });
  const receive = useCallback(
    (script: AuthorizedScript, operation: string) => {
      if (script.id !== id || operation === 'delete') return;
      const filter = { queryKey: ['getScript', { id }], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData(filter.queryKey, { data: script });
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('ScriptInfoUpdated', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('ScriptInfoUpdated', receive),
    [receive],
  );
  useRealtimeGroup({ groupName: `script:${id}`, setupEventListeners, removeEventListeners });
  return query;
}
