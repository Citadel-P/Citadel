import { randomUuid } from './utils';
import { collectionEventKey } from './collection-scope';
import { RealtimeConnection, RealtimeConnectionState, RealtimeConnectionOptions } from './realtime-connection';

// Transport-only compatibility boundary. Resource hooks retain their existing
// on/off/invoke contract; event names and payloads are owned by the server.
export function createWebSocketConnection(
  { baseUrl, accessTokenFactory }: RealtimeConnectionOptions,
  socketFactory: (url: string) => WebSocket = (url) => new WebSocket(url),
): RealtimeConnection {
  type Handler = (...args: any[]) => void;
  const subscriptionIds = new Map<string, string>();
  const subscriptionScopes = new Map<string, string>();
  const handlers = new Map<string, Set<Handler>>();
  const pending = new Map<
    string,
    { resolve: (value: unknown) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }
  >();
  const reconnecting: Handler[] = [],
    reconnected: Handler[] = [],
    closed: Handler[] = [];
  let socket: WebSocket | undefined;
  let state: RealtimeConnectionState = RealtimeConnectionState.Disconnected;
  let stopped = false;
  let nextId = 0;
  let reconnectAttempt = 0;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let rejectStart: ((error: Error) => void) | undefined;
  const failPending = () => {
    for (const invocation of pending.values()) {
      clearTimeout(invocation.timer);
      invocation.reject(new Error('Realtime connection closed'));
    }
    pending.clear();
    subscriptionIds.clear();
    subscriptionScopes.clear();
  };
  const emit = (callbacks: Handler[], ...args: unknown[]) => callbacks.forEach((callback) => callback(...args));
  const start = (): Promise<void> => {
    stopped = false;
    state = RealtimeConnectionState.Connecting;
    const url = new URL('/api/v1/realtime', baseUrl || window.location.origin);
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    const current = socketFactory(url.toString());
    socket = current;
    return new Promise((resolve, reject) => {
      let connected = false;
      let subscribedAt = 0;
      const deadline = setTimeout(() => {
        reject(new Error('Realtime subscription timed out'));
        current.close();
      }, 15_000);
      rejectStart = (error) => {
        clearTimeout(deadline);
        reject(error);
      };
      current.addEventListener('open', () => {
        if (current !== socket || stopped) return;
        current.send(
          JSON.stringify({
            protocolVersion: 1,
            kind: 'subscribe',
            clientMode: 'groups',
            accessToken: accessTokenFactory(),
          }),
        );
      });
      current.addEventListener('message', (message) => {
        if (current !== socket || stopped) return;
        let value;
        try {
          value = JSON.parse(String(message.data));
        } catch {
          return;
        }
        if (value.protocolVersion !== 1) return;
        if (value.kind === 'subscribed') {
          if (connected) return;
          connected = true;
          subscribedAt = Date.now();
          clearTimeout(deadline);
          rejectStart = undefined;
          state = RealtimeConnectionState.Connected;
          resolve();
        } else if (value.kind === 'completion') {
          const invocation = pending.get(value.invocationId);
          if (!invocation) return;
          pending.delete(value.invocationId);
          clearTimeout(invocation.timer);
          if (value.error) invocation.reject(new Error(value.error));
          else invocation.resolve(value.result);
        } else if (value.kind === 'event' && typeof value.target === 'string' && Array.isArray(value.arguments)) {
          let key = value.target.toLowerCase();
          if (value.subscriptionId !== undefined) {
            const scope = subscriptionScopes.get(value.subscriptionId);
            if (!scope) return;
            key = collectionEventKey(scope, value.target);
          } else if (value.target === 'RealtimeGroupCapacity') {
            value.arguments[0] = subscriptionScopes.get(value.arguments[0]) ?? value.arguments[0];
          }
          handlers.get(key)?.forEach((handler) => handler(...value.arguments));
        }
      });
      current.addEventListener('close', () => {
        clearTimeout(deadline);
        if (current !== socket) return;
        socket = undefined;
        failPending();
        rejectStart = undefined;
        if (!connected) {
          state = RealtimeConnectionState.Disconnected;
          reject(new Error('Realtime subscription failed'));
          return;
        }
        if (stopped) return;
        state = RealtimeConnectionState.Reconnecting;
        emit(reconnecting);
        if (Date.now() - subscribedAt >= 30_000) reconnectAttempt = 0;
        const retry = () => {
          if (stopped) return;
          const base = [0, 2000, 5000, 10000, 30000][Math.min(reconnectAttempt++, 4)];
          retryTimer = setTimeout(
            () => {
              if (stopped) return;
              start()
                .then(() => emit(reconnected))
                .catch(() => {
                  if (stopped) return;
                  state = RealtimeConnectionState.Reconnecting;
                  retry();
                });
            },
            base * (0.75 + Math.random() * 0.25),
          );
        };
        retry();
      });
    });
  };
  const adapter: RealtimeConnection = {
    get state() {
      return state;
    },
    start,
    async stop() {
      stopped = true;
      reconnectAttempt = 0;
      clearTimeout(retryTimer);
      rejectStart?.(new Error('Realtime connection stopped'));
      rejectStart = undefined;
      const current = socket;
      socket = undefined;
      current?.close();
      failPending();
      state = RealtimeConnectionState.Disconnected;
      emit(closed);
    },
    on(name: string, handler: Handler) {
      const key = name.startsWith('[') ? name : name.toLowerCase();
      const callbacks = handlers.get(key) ?? new Set<Handler>();
      callbacks.add(handler);
      handlers.set(key, callbacks);
    },
    off(name: string, handler?: Handler) {
      const key = name.startsWith('[') ? name : name.toLowerCase();
      if (handler) handlers.get(key)?.delete(handler);
      if (!handler || !handlers.get(key)?.size) handlers.delete(key);
    },
    onreconnecting(handler: Handler) {
      reconnecting.push(handler);
    },
    onreconnected(handler: Handler) {
      reconnected.push(handler);
    },
    onclose(handler: Handler) {
      closed.push(handler);
    },
    invoke(target: string, ...args: unknown[]): Promise<unknown> {
      if (state !== RealtimeConnectionState.Connected || !socket)
        return Promise.reject(new Error('Realtime connection is not ready'));
      if (pending.size >= 128) return Promise.reject(new Error('Too many pending realtime invocations'));
      const scope = args[0];
      let ambiguousJoin: string | undefined;
      let release: (() => void) | undefined;
      if ((target === 'JoinGroup' || target === 'LeaveGroup') && typeof scope === 'string' && scope.startsWith('{')) {
        let id = subscriptionIds.get(scope);
        if (target === 'JoinGroup') {
          id ??= randomUuid();
          subscriptionIds.set(scope, id);
          subscriptionScopes.set(id, scope);
          ambiguousJoin = id;
          args = [{ ...JSON.parse(scope), subscriptionId: id }];
        } else {
          if (!id) return Promise.resolve();
          subscriptionIds.delete(scope);
          subscriptionScopes.delete(id);
          args = [id];
        }
        const joinedId = id;
        release = () => {
          if (subscriptionIds.get(scope) === joinedId) subscriptionIds.delete(scope);
          subscriptionScopes.delete(joinedId);
        };
      }
      const invokingSocket = socket;
      const invocationId = String(++nextId);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(invocationId);
          release?.();
          // A completion may be queued behind our timeout. Retire the event route
          // immediately and put a leave for the original UUID ahead of any retry.
          if (ambiguousJoin && socket === invokingSocket) {
            void adapter.invoke('LeaveGroup', ambiguousJoin).catch(() => {
              if (socket === invokingSocket) invokingSocket.close();
            });
          }
          reject(new Error('Realtime invocation timed out'));
        }, 15_000);
        pending.set(invocationId, {
          resolve,
          reject: (error) => {
            if (error.message !== 'Realtime snapshot limit exceeded') release?.();
            reject(error);
          },
          timer,
        });
        try {
          socket!.send(JSON.stringify({ protocolVersion: 1, kind: 'invoke', invocationId, target, arguments: args }));
        } catch (error) {
          clearTimeout(timer);
          pending.delete(invocationId);
          release?.();
          reject(error);
        }
      });
    },
  };
  return adapter;
}
