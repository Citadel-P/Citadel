import { createWebSocketConnection } from './createWebSocketConnection';
import { RealtimeConnectionOptions } from './realtime-connection';

export const isRealtimeTransportEnabled = (transport?: string) => transport === 'WebSocketV1';

export function createRealtimeConnection(
  transport: string | undefined,
  options: RealtimeConnectionOptions,
  socketFactory?: (url: string) => WebSocket,
) {
  if (transport === 'WebSocketV1') return createWebSocketConnection(options, socketFactory);
  throw new Error('Realtime transport is not enabled');
}
