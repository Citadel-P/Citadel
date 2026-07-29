import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

export interface SetupError {
  title: string;
  message: string;
}

export interface SetupContextValue {
  isSetupReady: boolean;
  requiresSetup: boolean;
  error?: SetupError;
  markSetupComplete: () => void;
  retry: () => void;
}

export const SetupContext = createContext<SetupContextValue | undefined>(undefined);
SetupContext.displayName = 'SetupContext';

export const useSetupContext = () => useRequiredContext(SetupContext);
