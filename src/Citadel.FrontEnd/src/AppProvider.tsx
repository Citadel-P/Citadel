import { useEffect, useState, useMemo, createContext } from 'react';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { AppPaths } from '@/AppRoutes';
import { ContainerInfoView, PlatformView } from './api/_generated';
import { useGETPlatform } from './features/platforms/hooks/useGETPlatform';
import { useGETContainerInfo } from './features/containers/hooks/useGETContainerInfo';
import { useRequiredContext } from './hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
  route: { path: string };
  currentPlatform: PlatformView | undefined;
  currentContainer: ContainerInfoView | undefined;
}

export const AppContext = createContext<IContext | undefined>(undefined);

const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const { platformId, containerId } = useParams();

  // Fetch platform and container data
  const { data: platformData, isLoading: platformIsLoading, isSuccess: platformIsSuccess } = useGETPlatform(platformId);
  const {
    data: containerData,
    isLoading: containerIsLoading,
    isSuccess: containerIsSuccess,
  } = useGETContainerInfo(containerId);

  // State variables
  const [currentPlatform, setCurrentPlatform] = useState<PlatformView | undefined>(undefined);
  const [currentContainer, setCurrentContainer] = useState<ContainerInfoView | undefined>(undefined);

  // Memoized route
  const [{ route }] = useMemo(
    () =>
      matchRoutes(
        Object.values(AppPaths).map((s) => ({ path: s })),
        location,
      ) || [{ route: { path: '' } }],
    [location],
  );

  // Memoized loading state
  const isLoading = useMemo(() => platformIsLoading || containerIsLoading, [platformIsLoading, containerIsLoading]);

  useEffect(() => {
    const shouldClearContainer = !containerId && currentContainer !== undefined;
    const shouldClearPlatform = !platformId && !containerId && currentPlatform !== undefined;

    if (shouldClearContainer) {
      setCurrentContainer(undefined);
    }

    if (shouldClearPlatform) {
      setCurrentPlatform(undefined);
    }

    // Set platform data from platform API
    if (platformIsSuccess && platformData?.data?.id) {
      setCurrentPlatform((prevPlatform) => {
        if (!prevPlatform || prevPlatform.id !== platformData.data.id) {
          return platformData.data;
        }
        return prevPlatform;
      });
    }

    // Set container and its platform data if available
    if (containerIsSuccess && containerData?.data) {
      const container = containerData.data;

      // Update container if changed
      if (!currentContainer || currentContainer.id !== container.id) {
        setCurrentContainer(container);
      }

      // Update platform from container data if it exists
      if (container.platformId && (!currentPlatform || currentPlatform.id !== container.platformId)) {
        setCurrentPlatform(undefined);
      }
    }
  }, [
    platformId,
    containerId,
    platformData?.data,
    containerData?.data,
    platformIsSuccess,
    containerIsSuccess,
    currentPlatform,
    currentContainer,
  ]);

  const contextValue = useMemo(
    () => ({
      route,
      isLoading,
      currentPlatform,
      currentContainer,
    }),
    [route, isLoading, currentPlatform, currentContainer],
  );

  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};

export default AppProvider;
export const useAppContext = () => useRequiredContext(AppContext);
