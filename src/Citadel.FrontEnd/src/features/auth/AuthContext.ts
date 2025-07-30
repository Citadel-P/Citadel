import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  accessToken: string | undefined;
  isAuthenticated: boolean;
}

export const AuthContext = createContext<IContext | undefined>(undefined);
AuthContext.displayName = 'AuthContext';

export const useAuthContext = () => useRequiredContext(AuthContext);
