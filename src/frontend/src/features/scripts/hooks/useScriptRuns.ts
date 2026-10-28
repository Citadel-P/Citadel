import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ScriptRunView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useScriptRuns(scriptId: string) {
  const queryClient = useQueryClient();
  const args = useMemo(() => ({ id: scriptId, query: { limit: 50 } }), [scriptId]);
  const query = useRead('listScriptRuns', args);
  const receiveWindow = useCallback(
    (snapshot: { scriptId: string; ids: string[] }) => {
      if (snapshot.scriptId !== scriptId) return;
      const filter = { queryKey: ['listScriptRuns', args], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData<{ data: { runs: ScriptRunView[] } }>(filter.queryKey, (current) => ({
        data: { runs: (current?.data.runs ?? []).filter((run) => snapshot.ids.includes(run.id)) },
      }));
    },
    [scriptId, args, queryClient],
  );
  const receive = useCallback(
    (run: ScriptRunView, operation: string) => {
      if (run.scriptId !== scriptId || operation === 'delete') return;
      const filter = { queryKey: ['listScriptRuns', args], exact: true };
      // Cancel the older GET before updating the authoritative live window.
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData<{ data: { runs: ScriptRunView[] } }>(filter.queryKey, (current) => ({
        data: {
          runs: [...(current?.data.runs ?? []).filter((item) => item.id !== run.id), run]
            .sort((a, b) => String(b.queuedAt).localeCompare(String(a.queuedAt)) || b.id.localeCompare(a.id))
            .slice(0, 50),
        },
      }));
    },
    [scriptId, args, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('ScriptRunWindow', receiveWindow);
      connection.on('ScriptRunInfoUpdated', receive);
    },
    [receiveWindow, receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('ScriptRunWindow', receiveWindow);
      connection.off('ScriptRunInfoUpdated', receive);
    },
    [receiveWindow, receive],
  );
  useRealtimeGroup({
    groupName: `scripts-runs:${scriptId}`,
    setupEventListeners,
    removeEventListeners,
  });
  return query;
}
