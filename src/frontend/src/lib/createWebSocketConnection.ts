import { RealtimeConnection, RealtimeConnectionState, RealtimeConnectionOptions } from './realtime-connection';

// Transport-only compatibility boundary. Resource hooks retain their existing
// on/off/invoke contract; event names and payloads are owned by the server.
export function createWebSocketConnection(
  { baseUrl, accessTokenFactory }: RealtimeConnectionOptions,
  socketFactory: (url: string) => WebSocket = (url) => new WebSocket(url),
): RealtimeConnection {
  type Handler = (...args: any[]) => void;
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
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let rejectStart: ((error: Error) => void) | undefined;
  const failPending = () => {
    for (const invocation of pending.values()) {
      clearTimeout(invocation.timer);
      invocation.reject(new Error('Realtime connection closed'));
    }
    pending.clear();
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
          connected = true;
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
          handlers.get(value.target.toLowerCase())?.forEach((handler) => handler(...value.arguments));
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
        let attempt = 0;
        const retry = () => {
          if (stopped) return;
          retryTimer = setTimeout(() => {
            if (stopped) return;
            start()
              .then(() => emit(reconnected))
              .catch(() => {
                if (stopped) return;
                state = RealtimeConnectionState.Reconnecting;
                attempt++;
                retry();
              });
          }, [0, 2_000, 5_000, 10_000, 30_000][Math.min(attempt, 4)]);
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
      const key = name.toLowerCase();
      const callbacks = handlers.get(key) ?? new Set<Handler>();
      callbacks.add(handler);
      handlers.set(key, callbacks);
    },
    off(name: string, handler?: Handler) {
      const key = name.toLowerCase();
      if (handler) handlers.get(key)?.delete(handler);
      else handlers.delete(key);
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
      const invocationId = String(++nextId);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(invocationId);
          reject(new Error('Realtime invocation timed out'));
        }, 15_000);
        pending.set(invocationId, { resolve, reject, timer });
        try {
          socket!.send(JSON.stringify({ protocolVersion: 1, kind: 'invoke', invocationId, target, arguments: args }));
        } catch (error) {
          clearTimeout(timer);
          pending.delete(invocationId);
          reject(error);
        }
      });
    },
  };
  return adapter;
}
