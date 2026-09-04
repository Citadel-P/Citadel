import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { createContext } from 'react';
import { useRequiredContext } from '../../hooks/useRequiredContext';

export type LiveConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected' | 'offline';

type SignalRGroupTransport = {
  connection: HubConnection | null;
  connectionState: HubConnectionState;
  joinGroup: (groupName: string, setup?: (hub: HubConnection) => void) => Promise<void>;
  leaveGroup: (groupName: string, remove?: (hub: HubConnection) => void) => Promise<void>;
};

export type RealtimeContextType = {
  signalR?: SignalRGroupTransport;
  liveConnectionState: LiveConnectionState;
  interruptedAt?: number;
  lastConnectedAt?: number;
  retryConnection: () => Promise<void>;
};

export const RealtimeContext = createContext<RealtimeContextType | undefined>(undefined);
RealtimeContext.displayName = 'RealtimeContext';

export const useRealtimeContext = () => useRequiredContext(RealtimeContext);
