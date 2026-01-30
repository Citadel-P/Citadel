import { createContext } from 'react';
import { useRequiredContext } from '../../hooks/useRequiredContext';
import { PlatformView } from '../../api/generated/api.types';

interface IContext {
  isLoading: boolean;
  currentPlatform: PlatformView | undefined;
}

export const AppContext = createContext<IContext | undefined>(undefined);
AppContext.displayName = 'AppContext';

export const useAppContext = () => useRequiredContext(AppContext);
