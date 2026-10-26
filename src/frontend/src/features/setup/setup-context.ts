import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { MfaPolicy } from '@/api/generated/api.types';

export interface SetupError {
  title: string;
  message: string;
}

export interface SetupContextValue {
  passwordMinimumLength: number;
  passwordMaximumLength: number;
  updateChecksEnabled: boolean;
  updateChecksAllowed: boolean;
  mfaPolicy: MfaPolicy;
  minimumMfaPolicy: MfaPolicy;
  isSetupReady: boolean;
  requiresSetup: boolean;
  error?: SetupError;
  markSetupComplete: () => void;
  retry: () => void;
}

export const SetupContext = createContext<SetupContextValue | undefined>(undefined);
SetupContext.displayName = 'SetupContext';

export const useSetupContext = () => useRequiredContext(SetupContext);
