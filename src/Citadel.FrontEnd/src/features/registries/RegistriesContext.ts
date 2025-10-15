import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { DeleteRegistriesInput, RegistryView } from '@/api/generated/api.types';
import { IDialogData } from '@/lib/hooks';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[] | undefined;
  selectedRows: RegistryView[] | undefined;
  setSelectedRows: (ids: RegistryView[]) => void;
  requestDelete: (ids: DeleteRegistriesInput) => void;
  deleteIsPending: boolean;
  dialogData: IDialogData<RegistryView>;
  setDialogData: (data: IDialogData<RegistryView>) => void;
}

export const RegistriesContext = createContext<IContext | undefined>(undefined);
RegistriesContext.displayName = 'RegistriesContext';

export const useRegistriesContext = () => useRequiredContext(RegistriesContext);
