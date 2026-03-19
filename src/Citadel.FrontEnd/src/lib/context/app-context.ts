import { createContext } from 'react';
import { useRequiredContext } from '../../hooks/useRequiredContext';
import { AlertEventView, PlatformView } from '../../api/generated/api.types';

interface IContext {
  isLoading: boolean;
  currentPlatform: PlatformView | undefined;
  unresolvedAlertCount: number;
  liveAlertEvents: Record<string, AlertEventView>;
  receivedAlertEventIds: string[];
}

export const AppContext = createContext<IContext | undefined>(undefined);
AppContext.displayName = 'AppContext';

export const useAppContext = () => useRequiredContext(AppContext);
