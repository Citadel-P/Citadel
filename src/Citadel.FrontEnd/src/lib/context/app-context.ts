import type { ResourceReadState } from '@/components/custom/resource-read-error';
import { createContext } from 'react';
import { useRequiredContext } from '../../hooks/useRequiredContext';
import { AlertEventView, ApplicationInfoView, PlatformView } from '../../api/generated/api.types';

interface IContext {
  isLoading: boolean;
  platformRead?: ResourceReadState;
  currentPlatform: PlatformView | undefined;
  platforms: PlatformView[] | undefined;
  applicationInfo: ApplicationInfoView | undefined;
  unresolvedAlertCount: number;
  liveAlertEvents: Record<string, AlertEventView>;
  receivedAlertEventIds: string[];
}

export const AppContext = createContext<IContext | undefined>(undefined);
AppContext.displayName = 'AppContext';

export const useAppContext = () => useRequiredContext(AppContext);
