import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext, JSX } from 'react';
import { RegistryInput, RegistryWithConfigView } from '@/api/_generated';

export interface IContext {
  mode: FormMode;
  formTitle: string;
  isLoading: boolean;
  isLoadingForm: boolean;
  saveButtonTitle: string;
  validationErrors: string | undefined | null;
  registry: RegistryWithConfigView | undefined;
  providers: IRegistryProvider[];
  currentProvider: string;
  setCurrentProvider: (value: string) => void;
  onPostForm: (values: RegistryInput | Partial<RegistryInput>) => void;
}
export interface IRegistryProvider {
  id: string;
  name: string;
  description: string;
  configuration: JSX.Element;
  disabled: boolean;
}

export type FormMode = 'edit' | 'add';

export const RegistryFormContext = createContext<IContext | undefined>(undefined);
RegistryFormContext.displayName = 'RegistryFormContext';

export const useRegistryFormContext = () => useRequiredContext(RegistryFormContext);
