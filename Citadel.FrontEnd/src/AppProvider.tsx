import { useEffect, useState } from 'react';
import { createContext } from 'use-context-selector';
import { ContainerInfoView, PlatformView } from './api/_generated';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { useGETPlatform } from './features/platforms/hooks/useGETPlatform';
import { paths } from '@/AppRoutes';
import { useGETContainer } from './features/containers/hooks/useGETContainer';
import { useGETPlatforms } from './features/platforms/hooks/useGETPlatforms';
import usePlatformHub from './features/platforms/hooks/usePlatformHub';

interface IContext {
  route: { path: string };
  isBreadcrumbHidden: boolean;
  isLoading: boolean;
  setIsBreadcrumbHidden: (s: boolean) => void;
  platforms: PlatformView[] | undefined;
  currentPlatform: PlatformView | undefined;
  currentContainer: ContainerInfoView | undefined;
}

interface IProps {
  children?: React.ReactNode;
}

export const AppContext = createContext<IContext | undefined>(undefined);

const AppProvider: React.FC<IProps> = ({ children }) => {
  const { platformId, containerId } = useParams();

  const { platformsMessage } = usePlatformHub();
  const { data, isLoading, isSuccess } = useGETPlatforms();

  const [isBreadcrumbHidden, setIsBreadcrumbHidden] = useState(false);
  const { data: platformData, isSuccess: platformIsSuccess } = useGETPlatform(platformId);
  const { data: containerData, isSuccess: containerIsSuccess } = useGETContainer(containerId);
  const [currentPlatform, setCurrentPlatform] = useState<PlatformView | undefined>(undefined);
  const [currentContainer, setCurrentContainer] = useState<ContainerInfoView | undefined>(undefined);
  const location = useLocation();
  const [{ route }] = matchRoutes(
    paths.map((s) => ({ path: s })),
    location,
  );
  let platforms: PlatformView[] | undefined;

  useEffect(() => {
    if (platformIsSuccess && platformData?.data) {
      setCurrentPlatform(platformData?.data);
    }
  }, [platformData, platformIsSuccess]);

  useEffect(() => {
    if (platformsMessage) {
      setCurrentPlatform((p) => platformsMessage?.find((s) => s.address === p?.address));
    }
  }, [platformsMessage]);

  useEffect(() => {
    if (containerIsSuccess && containerData?.data?.platform) {
      setCurrentContainer(containerData?.data);
      setCurrentPlatform(containerData?.data.platform);
    }
  }, [containerData, containerIsSuccess]);

  if (isSuccess && data?.data) {
    platforms = data.data.platforms!;
  }

  if (platformsMessage) {
    platforms = platformsMessage;
  }

  return (
    <AppContext.Provider
      value={{
        route,
        isBreadcrumbHidden,
        setIsBreadcrumbHidden,
        platforms,
        isLoading,
        currentPlatform,
        currentContainer,
      }}>
      {children}
    </AppContext.Provider>
  );
};

export default AppProvider;
