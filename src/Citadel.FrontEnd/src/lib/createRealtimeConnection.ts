import { createSignalRConnection } from './createSignalRConnection';
import { createWebSocketConnection } from './createWebSocketConnection';
import { RealtimeConnectionOptions } from './realtime-connection';

export const isRealtimeTransportEnabled = (transport?: string) =>
  transport === 'WebSocketV1' || transport === 'SignalR';

export function createRealtimeConnection(
  transport: string | undefined,
  options: RealtimeConnectionOptions,
  socketFactory?: (url: string) => WebSocket,
) {
  if (transport === 'WebSocketV1') return createWebSocketConnection(options, socketFactory);
  // Retained only for the .NET backend during migration.
  if (transport === 'SignalR') return createSignalRConnection(options);
  throw new Error('Realtime transport is not enabled');
}
