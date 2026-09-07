import { RealtimeConnection, RealtimeConnectionState } from '@/lib/realtime-connection';
import { createContext } from 'react';
import { useRequiredContext } from '../../hooks/useRequiredContext';

export type LiveConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected' | 'offline';

type RealtimeGroupTransport = {
  connection: RealtimeConnection | null;
  connectionState: RealtimeConnectionState;
  joinGroup: (groupName: string, setup?: (hub: RealtimeConnection) => void) => Promise<void>;
  leaveGroup: (groupName: string, remove?: (hub: RealtimeConnection) => void) => Promise<void>;
};

export type RealtimeContextType = {
  groups?: RealtimeGroupTransport;
  liveConnectionState: LiveConnectionState;
  interruptedAt?: number;
  lastConnectedAt?: number;
  retryConnection: () => Promise<void>;
};

export const RealtimeContext = createContext<RealtimeContextType | undefined>(undefined);
RealtimeContext.displayName = 'RealtimeContext';

export const useRealtimeContext = () => useRequiredContext(RealtimeContext);
