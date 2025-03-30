import { useEffect, useState, useMemo } from 'react';
import { createContext } from 'use-context-selector';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { AppPaths } from '@/AppRoutes';
import { ContainerInfoView, PlatformView } from './api/_generated';
import { useGETPlatform } from './features/platforms/hooks/useGETPlatform';
import { useGETContainer } from './features/containers/hooks/useGETContainer';

interface IContext {
  isLoading: boolean;
  route: { path: string };
  isBreadcrumbHidden: boolean;
  setIsBreadcrumbHidden: (s: boolean) => void;
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
  } = useGETContainer(containerId);

  // State variables
  const [currentPlatform, setCurrentPlatform] = useState<PlatformView | undefined>(undefined);
  const [currentContainer, setCurrentContainer] = useState<ContainerInfoView | undefined>(undefined);
  const [isBreadcrumbHidden, setIsBreadcrumbHidden] = useState(false);

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

  // Update current platform when platform data is fetched
  useEffect(() => {
    if (platformIsSuccess && platformData?.data) {
      setCurrentPlatform((prevPlatform) =>
        prevPlatform?.id !== platformData.data.id ? platformData.data : prevPlatform,
      );
    }
  }, [platformIsSuccess, platformData]);

  // Update current container and platform when container data is fetched
  useEffect(() => {
    if (containerIsSuccess && containerData?.data?.platform) {
      setCurrentContainer((prevContainer) =>
        prevContainer?.id !== containerData.data.id ? containerData.data : prevContainer,
      );
      setCurrentPlatform((prevPlatform) =>
        prevPlatform?.id !== containerData?.data?.platform?.id ? containerData.data.platform : prevPlatform,
      );
    }
  }, [containerData, containerIsSuccess]);

  const contextValue = useMemo(
    () => ({
      route,
      isLoading,
      isBreadcrumbHidden,
      setIsBreadcrumbHidden,
      currentPlatform,
      currentContainer,
    }),
    [route, isLoading, isBreadcrumbHidden, currentPlatform, currentContainer],
  );

  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};

export default AppProvider;
