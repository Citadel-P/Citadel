import { createContext, useCallback, useEffect, useMemo, useState } from 'react';
import { PlatformView } from '@/api/_generated';
import usePlatformHub from './hooks/usePlatformHub';
import { useDELETEPlatform } from './hooks/useDELETEPlatform';
import { toast } from 'sonner';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
  requestDelete: (platformId: string) => void;
  deleteIsPending: boolean;
}

interface IDeleteDialogData {
  open: boolean;
  platform?: PlatformView | undefined;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);

const PlatformsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformsMessage, isLoading } = usePlatformHub();
  const [dialogData, setDialogData] = useState<IDeleteDialogData>({ open: false });
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEPlatform();

  const platforms = useMemo(() => {
    if (platformsMessage && platformsMessage.length > 0) {
      return platformsMessage;
    }
    return undefined;
  }, [platformsMessage]);

  // Request to delete platform
  const requestDelete = useCallback(
    (platformId: string) => {
      mutate({ id: platformId });
    },
    [mutate],
  );

  // Handle successful deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      setDialogData({ open: false });
      toast.success('The selected platform has been successfully deleted');
    }
  }, [deleteIsSuccess, setDialogData]);

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

export default PlatformsProvider;
export const usePlatformsContext = () => useRequiredContext(PlatformsContext);
