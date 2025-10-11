import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
}

export const DeploymentsContext = createContext<IContext | undefined>(undefined);
DeploymentsContext.displayName = 'DeploymentsContext';

export const useRegistriesContext = () => useRequiredContext(DeploymentsContext);
