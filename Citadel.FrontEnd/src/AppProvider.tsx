import { createContext, useEffect, useState } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { ContainerInfoView, PlatformView } from './api/_generated';
import { matchRoutes, useLocation, useParams } from 'react-router';
import { useGETPlatform } from './features/platforms/hooks/useGETPlatform';
import { paths } from '@/AppRoutes';
import { useGETContainer } from './features/containers/hooks/useGETContainer';

interface IContext {
  route: { path: string };
  isBreadcrumbHidden: boolean;
  setIsBreadcrumbHidden: (s: boolean) => void;
  currentPlatform: PlatformView | undefined;
  currentContainer: ContainerInfoView | undefined;
}

interface IProps {
  children?: React.ReactNode;
}

const AppContext = createContext<IContext | undefined>(undefined);

const AppProvider: React.FC<IProps> = ({ children }) => {
  const { platformId, containerId } = useParams();
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

  useEffect(() => {
    if (platformIsSuccess) {
      setCurrentPlatform(platformData?.data);
    }
  }, [platformData, platformIsSuccess]);

  useEffect(() => {
    if (containerIsSuccess) {
      setCurrentContainer(containerData?.data);
      setCurrentPlatform(containerData?.data.platform);
    }
  }, [containerData, containerIsSuccess]);

  return (
    <AppContext.Provider
      value={{
        route,
        isBreadcrumbHidden,
        setIsBreadcrumbHidden,
        currentPlatform,
        currentContainer,
      }}>
      {children}
    </AppContext.Provider>
  );
};

export default AppProvider;

export const useAppContext = () => useRequiredContext(AppContext);
