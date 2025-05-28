import { createContext } from 'use-context-selector';
import { useMemo } from 'react';
import { PlatformView } from '@/api/_generated';
import usePlatformHub from './hooks/usePlatformHub';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);

const PlatformsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformsMessage, isLoading } = usePlatformHub();

  const platforms = useMemo(() => {
    if (platformsMessage && platformsMessage.length > 0) {
      return platformsMessage;
    }
    return undefined;
  }, [platformsMessage]);

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
