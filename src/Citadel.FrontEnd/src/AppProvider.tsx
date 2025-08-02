import { useMemo } from 'react';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { AppPaths } from '@/AppRoutes';
import { useGETPlatform } from './features/platforms/hooks/useGETPlatform';
import { useGETContainerInfo } from './features/containers/hooks/useGETContainerInfo';
import { AppContext } from './AppContext';

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const { platformId, containerId } = useParams();
  const { data: platformData, isLoading: platformIsLoading } = useGETPlatform(platformId);
  const { data: containerData, isLoading: containerIsLoading } = useGETContainerInfo(containerId);

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

  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};
