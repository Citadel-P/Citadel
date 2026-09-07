export const RealtimeConnectionState = {
  Disconnected: 'Disconnected',
  Connecting: 'Connecting',
  Connected: 'Connected',
  Disconnecting: 'Disconnecting',
  Reconnecting: 'Reconnecting',
} as const;

export type RealtimeConnectionState = (typeof RealtimeConnectionState)[keyof typeof RealtimeConnectionState];

// The event/group contract shared by Citadel transports, not a SignalR protocol.
export interface RealtimeConnection {
  readonly state: RealtimeConnectionState;
  start(): Promise<void>;
  stop(): Promise<void>;
  on(name: string, handler: (...args: any[]) => void): void;
  off(name: string, handler?: (...args: any[]) => void): void;
  onreconnecting(handler: (error?: Error) => void): void;
  onreconnected(handler: (connectionId?: string) => void): void;
  onclose(handler: (error?: Error) => void): void;
  invoke(target: string, ...args: unknown[]): Promise<unknown>;
}

export type RealtimeConnectionOptions = {
  baseUrl: string;
  accessTokenFactory: () => string;
};

export type RealtimeConnectionFactory = (options: RealtimeConnectionOptions) => RealtimeConnection;
