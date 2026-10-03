import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { Api } from './generated/api.types';

interface IContext {
  apiClient: Api<unknown>;
}

export const ApiClientContext = createContext<IContext | undefined>(undefined);
ApiClientContext.displayName = 'ApiClientContext';

export const useApiClientContext = () => useRequiredContext(ApiClientContext);
