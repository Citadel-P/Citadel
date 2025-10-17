import { createContext } from 'react';
import { useRequiredContext } from './hooks/useRequiredContext';
import { ContainerInfoView, PlatformView } from './api/generated/api.types';

interface IContext {
  isLoading: boolean;
  route: { path: string };
  currentPlatform: PlatformView | undefined;
  currentContainer: ContainerInfoView | undefined;
}

export const AppContext = createContext<IContext | undefined>(undefined);
AppContext.displayName = 'AppContext';

export const useAppContext = () => useRequiredContext(AppContext);
