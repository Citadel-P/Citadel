import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

export interface IContext {
  mode: FormMode;
  formTitle: string;
  isLoading: boolean;
  isLoadingForm: boolean;
  saveButtonTitle: string;
  validationErrors: string | undefined | null;
}

export type FormMode = 'edit' | 'add';

export const DeploymentFormContext = createContext<IContext | undefined>(undefined);
DeploymentFormContext.displayName = 'DeploymentFormContext';

export const useDeploymentFormContext = () => useRequiredContext(DeploymentFormContext);
