import { useMemo } from 'react';
import { useParams } from 'react-router';
import { AppContext } from './app-context';
import { SignalRProvider } from './signalr-provider';
import { useRead } from '../hooks';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';
import { useAlertEventsGroup } from '@/features/alerters/alert-events/hooks/useAlertEventsGroup';

const AppProviderContent: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId, type, id } = useParams();
  const selectedPlatformId = platformId ?? (type === 'platforms' ? id : undefined);
  const { data: platformData, isLoading: platformIsLoading } = useRead('getPlatfom', { id: selectedPlatformId });
  const { platformsMessage, isLoading: platformsIsLoading } = usePlatformsGroup({ useTagFilter: false });
  const alertEventsGroup = useAlertEventsGroup();

  const currentPlatform = useMemo(() => {
    if (!selectedPlatformId) return platformData?.data;

    const livePlatform = platformsMessage?.find((platform) => platform.id === selectedPlatformId);
    if (livePlatform) return livePlatform;

    return platformData?.data;
  }, [selectedPlatformId, platformsMessage, platformData]);

  const contextValue = useMemo(
    () => ({
      isLoading: platformIsLoading || platformsIsLoading,
      currentPlatform,
      platforms: platformsMessage,
      ...alertEventsGroup,
    }),
    [platformIsLoading, platformsIsLoading, currentPlatform, platformsMessage, alertEventsGroup],
  );
  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <SignalRProvider>
    <AppProviderContent>{children}</AppProviderContent>
  </SignalRProvider>
);
