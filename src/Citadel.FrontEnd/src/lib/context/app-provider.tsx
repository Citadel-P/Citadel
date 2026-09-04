import { useMemo } from 'react';
import { useParams } from 'react-router';
import { AppContext } from './app-context';
import { RealtimeProvider } from './realtime-provider';
import { useRead } from '../hooks';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';
import { useAlertEventsGroup } from '@/features/alerters/alert-events/hooks/useAlertEventsGroup';
import { ApplicationInfoView } from '@/api/generated/api.types';

const AppProviderContent: React.FC<{
  children?: React.ReactNode;
  applicationInfo: ApplicationInfoView | undefined;
}> = ({ children, applicationInfo }) => {
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
      applicationInfo,
      ...alertEventsGroup,
    }),
    [platformIsLoading, platformsIsLoading, currentPlatform, platformsMessage, applicationInfo, alertEventsGroup],
  );
  return <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>;
};

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { data: applicationInfo } = useRead('getApplicationInfo');

  return (
    <RealtimeProvider realtimeTransport={applicationInfo?.data.realtimeTransport}>
      <AppProviderContent applicationInfo={applicationInfo?.data}>{children}</AppProviderContent>
    </RealtimeProvider>
  );
};
