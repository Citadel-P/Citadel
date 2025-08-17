import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { IDialogData } from '@/hooks/useDialogState';
import { RegistryView } from '@/api/_generated';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[] | undefined;
  selectedRows: RegistryView[] | undefined;
  setSelectedRows: (ids: RegistryView[]) => void;
  requestDelete: (ids: string[]) => void;
  deleteIsPending: boolean;
  dialogData: IDialogData<RegistryView>;
  setDialogData: (data: IDialogData<RegistryView>) => void;
}

export const RegistriesContext = createContext<IContext | undefined>(undefined);
RegistriesContext.displayName = 'RegistriesContext';

export const useRegistriesContext = () => useRequiredContext(RegistriesContext);
