import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { PlatformView } from '@/api/_generated';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
  requestDelete: (platformId: string) => void;
  deleteIsPending: boolean;
}

export interface IDeleteDialogData {
  open: boolean;
  platform?: PlatformView | undefined;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);
PlatformsContext.displayName = 'PlatformsContext';

export const usePlatformsContext = () => useRequiredContext(PlatformsContext);
