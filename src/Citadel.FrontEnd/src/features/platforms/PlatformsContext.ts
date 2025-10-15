import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { DeletePlatformsInput, PlatformView } from '@/api/generated/api.types';
import { IDialogData } from '@/lib/hooks';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
  dialogData: IDialogData<PlatformView>;
  setDialogData: (data: IDialogData<PlatformView>) => void;
  requestDelete: (ids: DeletePlatformsInput) => void;
  deleteIsPending: boolean;
}

export const PlatformsContext = createContext<IContext | undefined>(undefined);
PlatformsContext.displayName = 'PlatformsContext';

export const usePlatformsContext = () => useRequiredContext(PlatformsContext);
