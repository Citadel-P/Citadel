import { useMemo } from 'react';
import { useParams } from 'react-router';
import { AppContext } from './app-context';
import { SignalRProvider } from './signalr-provider';
import { useRead } from '../hooks';

export const AppProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId, containerId } = useParams();
  const { data: platformData, isLoading: platformIsLoading } = useRead('getPlatfom', { id: platformId });
  const { data: containerData, isLoading: containerIsLoading } = useRead('getContainerInfo', { id: containerId });

  const contextValue = useMemo(
    () => ({
      isLoading: platformIsLoading || containerIsLoading,
      currentPlatform: platformData?.data,
      currentContainer: containerData?.data,
    }),
    [platformIsLoading, containerIsLoading, platformData, containerData],
  );

  return (
    <AppContext.Provider value={contextValue}>
      <SignalRProvider>{children}</SignalRProvider>
    </AppContext.Provider>
  );
};
