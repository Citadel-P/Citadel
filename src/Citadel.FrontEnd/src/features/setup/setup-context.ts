import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

export interface SetupContextValue {
  isSetupReady: boolean;
  requiresSetup: boolean;
  error?: string;
  markSetupComplete: () => void;
  retry: () => void;
}

export const SetupContext = createContext<SetupContextValue | undefined>(undefined);
SetupContext.displayName = 'SetupContext';

export const useSetupContext = () => useRequiredContext(SetupContext);
