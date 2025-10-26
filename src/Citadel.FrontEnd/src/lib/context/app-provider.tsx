import { useMemo } from 'react';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { AppPaths } from '@/app-routes';
import { AppContext } from './app-context';
import { SignalRProvider } from './signalr-provider';
import { useRead } from '../hooks';

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const { platformId, containerId } = useParams();
  const { data: platformData, isLoading: platformIsLoading } = useRead('getPlatfom', { id: platformId });
  const { data: containerData, isLoading: containerIsLoading } = useRead('getContainerInfo', { id: containerId });

  const [{ route }] = useMemo(
    () =>
      matchRoutes(
        Object.values(AppPaths).map((s) => ({ path: s })),
        location,
      ) || [{ route: { path: '' } }],
    [location],
  );

  const contextValue = useMemo(
    () => ({
      route,
      isLoading: platformIsLoading || containerIsLoading,
      currentPlatform: platformData?.data,
      currentContainer: containerData?.data,
    }),
    [route, platformIsLoading, containerIsLoading, platformData, containerData],
  );

  return (
    <AppContext.Provider value={contextValue}>
      <SignalRProvider>{children}</SignalRProvider>
    </AppContext.Provider>
  );
};
