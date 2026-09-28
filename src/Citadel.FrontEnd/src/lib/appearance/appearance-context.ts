import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import type { AppearancePreferences } from './appearance-types';

export interface AppearanceContextValue {
  preferences: AppearancePreferences;
  effectiveMode: 'light' | 'dark';
  setMode: (value: AppearancePreferences['mode']) => void;
  setColor: (value: AppearancePreferences['color']) => void;
  setFont: (value: AppearancePreferences['font']) => void;
  setRadius: (value: AppearancePreferences['radius']) => void;
  setContentLayout: (value: AppearancePreferences['contentLayout']) => void;
  setDensity: (value: AppearancePreferences['density']) => void;
  resetAppearance: () => void;
}
export const AppearanceContext = createContext<AppearanceContextValue | undefined>(undefined);
export const useAppearance = () => useRequiredContext(AppearanceContext);
