import { createContext } from 'react';
import { Api } from './_generated';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  apiClient: Api<unknown>;
}

export const ApiClientContext = createContext<IContext | undefined>(undefined);
ApiClientContext.displayName = 'ApiClientContext';

export const useApiClientContext = () => useRequiredContext(ApiClientContext);
