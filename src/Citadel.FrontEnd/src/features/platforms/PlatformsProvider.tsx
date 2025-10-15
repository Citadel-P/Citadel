import { useMemo } from 'react';
import { usePlatformsGroup } from './hooks/usePlatformsGroup';
import { PlatformsContext } from './PlatformsContext';
import { PlatformView } from '@/api/generated/api.types';
import { useDeleteDialog } from '@/lib/hooks';

export const PlatformsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformsMessage, isLoading } = usePlatformsGroup();
  const { setDialogData, dialogData, requestDelete, deleteIsPending } = useDeleteDialog<PlatformView>({
    type: 'Platform',
  });

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
      dialogData,
      setDialogData,
      deleteIsPending,
      requestDelete,
    }),
    [isLoading, platforms, dialogData, setDialogData, requestDelete, deleteIsPending],
  );

  return <PlatformsContext.Provider value={contextValue}>{children}</PlatformsContext.Provider>;
};
