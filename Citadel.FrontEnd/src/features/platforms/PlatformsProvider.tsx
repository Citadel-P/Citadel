import { createContext } from 'use-context-selector';
import { useMemo } from 'react';
import { PlatformView } from '@/api/_generated';
import usePlatformHub from './hooks/usePlatformHub';
import { useGETPlatforms } from './hooks/useGETPlatforms';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);

const PlatformsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { data: platformsData, isLoading, isSuccess } = useGETPlatforms();
  const { platformsMessage } = usePlatformHub();

  const platforms = useMemo(() => {
    return isSuccess && platformsData?.data?.platforms ? platformsData.data.platforms : platformsMessage;
  }, [platformsMessage, isSuccess, platformsData]);

  // Memoized context value
  const contextValue = useMemo(
    () => ({
      isLoading,
      platforms,
    }),
    [isLoading, platforms],
  );

  return <PlatformsContext.Provider value={contextValue}>{children}</PlatformsContext.Provider>;
};

export default PlatformsProvider;
