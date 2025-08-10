import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { createContext } from 'react';
import { useRequiredContext } from './hooks/useRequiredContext';

export type SignalRContextType = {
  connection: HubConnection | null;
  connectionState: HubConnectionState;
  joinGroup: (groupName: string, setup?: (hub: HubConnection) => void) => Promise<void>;
  leaveGroup: (groupName: string, remove?: (hub: HubConnection) => void) => Promise<void>;
};

export const SignalRContext = createContext<SignalRContextType | undefined>(undefined);
SignalRContext.displayName = 'SignalRContext';

export const useSignalRContext = () => useRequiredContext(SignalRContext);
