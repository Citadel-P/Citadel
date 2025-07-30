import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { IDeleteDialogData } from '@/hooks/useDialogState';
import { RegistryView } from '@/api/_generated';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[] | undefined;
  selectedRows: RegistryView[] | undefined;
  setSelectedRows: (ids: RegistryView[]) => void;
  requestDelete: (ids: string[]) => void;
  deleteIsPending: boolean;
  dialogData: IDeleteDialogData<RegistryView>;
  setDialogData: (data: IDeleteDialogData<RegistryView>) => void;
}

export const RegistriesContext = createContext<IContext | undefined>(undefined);
RegistriesContext.displayName = 'RegistriesContext';

export const useRegistriesContext = () => useRequiredContext(RegistriesContext);
