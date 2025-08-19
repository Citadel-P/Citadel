import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { PlatformView } from '@/api/_generated';
import { IDialogData } from '@/hooks/useDialogState';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
  dialogData: IDialogData<PlatformView>;
  setDialogData: (data: IDialogData<PlatformView>) => void;
  requestDelete: (platformId: string) => void;
  deleteIsPending: boolean;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);
PlatformsContext.displayName = 'PlatformsContext';

export const usePlatformsContext = () => useRequiredContext(PlatformsContext);
