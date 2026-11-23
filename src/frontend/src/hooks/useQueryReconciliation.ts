import { collectionGroup } from '@/lib/collection-scope';
import { replaceEqualDeep, useQueryClient, type Query } from '@tanstack/react-query';
import { useCallback, useContext } from 'react';
import { RealtimeContext } from '@/lib/context/realtime-context';
import type { KnownResourceName } from '@/api/types';

// These lists keep live rows separately. A fallback GET must replace them even
// when its rows are identical to the last HTTP baseline.
export function useCollectionStructuralSharing(group: string) {
  const isGroupLive = useContext(RealtimeContext)?.groups?.isGroupLive;
  return useCallback(
    (previous: unknown, next: unknown) => (isGroupLive?.(group) ? replaceEqualDeep(previous, next) : next),
    [group, isGroupLive],
  );
}

// Check membership when the mutation settles. The socket or the subscription may have changed during the request.
export function useQueryReconciliation() {
  const client = useQueryClient();
  const isGroupLive = useContext(RealtimeContext)?.groups?.isGroupLive;
  return useCallback(
    async (group: string, operation: KnownResourceName, args?: object) => {
      const queryKey = args ? [operation, args] : [operation];
      const predicate = (query: Query) => {
        const identity = query.queryKey[1] as
          { query?: { tags?: string[]; platformId?: string; includeDisabled?: boolean } } | undefined;
        const scope = identity?.query;
        const name = collectionGroup(group, scope?.tags ?? [], scope?.platformId, scope?.includeDisabled);
        return !isGroupLive?.(name);
      };
      await client.cancelQueries({ queryKey, predicate, fetchStatus: 'fetching' });
      await client.invalidateQueries({ queryKey, predicate });
    },
    [client, isGroupLive],
  );
}

export function useResourceReconciliation(
  listGroup: string,
  listOperation: KnownResourceName,
  detailGroup: string,
  detailOperation: KnownResourceName,
) {
  const reconcile = useQueryReconciliation();
  return useCallback(
    async (id?: string) => {
      await Promise.all([
        reconcile(listGroup, listOperation),
        id && reconcile(`${detailGroup}:${id}`, detailOperation, { id }),
      ]);
    },
    [reconcile, listGroup, listOperation, detailGroup, detailOperation],
  );
}
