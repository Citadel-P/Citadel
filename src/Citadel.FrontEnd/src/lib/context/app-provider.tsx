import { useMemo } from 'react';
import { useParams } from 'react-router';
import { AppContext } from './app-context';
import { SignalRProvider } from './signalr-provider';
import { useRead } from '../hooks';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';

const AppProviderContent: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId } = useParams();
  const { data: platformData, isLoading: platformIsLoading } = useRead('getPlatfom', { id: platformId });
  const { platformsMessage, isLoading: platformsIsLoading } = usePlatformsGroup();

  const currentPlatform = useMemo(() => {
    if (!platformId) return platformData?.data;

    const livePlatform = platformsMessage?.find((platform) => platform.id === platformId);
    return livePlatform ?? platformData?.data;
  }, [platformId, platformsMessage, platformData]);

  const contextValue = useMemo(
    () => ({
      isLoading: platformIsLoading || platformsIsLoading,
      currentPlatform,
    }),
    [platformIsLoading, platformsIsLoading, currentPlatform],
  );
  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <SignalRProvider>
    <AppProviderContent>{children}</AppProviderContent>
  </SignalRProvider>
);
