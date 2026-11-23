import type { RealtimeConnection } from './realtime-connection';

export function collectionGroup(group: string, tags: readonly string[], platformId?: string, includeDisabled = false) {
  if (!tags.length && !platformId && !includeDisabled) return group;
  return JSON.stringify({
    group,
    tags,
    ...(platformId ? { platformId } : {}),
    ...(includeDisabled ? { includeDisabled } : {}),
  });
}

// Keep scope identity case-sensitive. Only the protocol event names are case-insensitive.
export const collectionEventKey = (scope: string, event: string) => JSON.stringify([scope, event.toLowerCase()]);

export function collectionListeners(connection: RealtimeConnection, scope: string): RealtimeConnection {
  return {
    ...connection,
    on: (name, handler) => connection.on(collectionEventKey(scope, name), handler),
    off: (name, handler) => connection.off(collectionEventKey(scope, name), handler),
  };
}
