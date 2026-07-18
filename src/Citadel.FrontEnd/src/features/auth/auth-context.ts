import { LoginRequest, LoginResponse } from '@/api/generated/api.types';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  accessToken: string | undefined;
  isAuthenticated: boolean;
  isAuthReady: boolean;
  isPending: boolean;
  validationErrors: string | undefined;
  logout: () => void;
  login: (request: LoginRequest) => Promise<LoginResponse | undefined>;
  completeLogin: (accessToken: string) => void;
}

export const AuthContext = createContext<IContext | undefined>(undefined);
AuthContext.displayName = 'AuthContext';

export const useAuthContext = () => useRequiredContext(AuthContext);
