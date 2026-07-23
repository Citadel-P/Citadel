import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  theme: ITheme;
  sidebarMinimized: boolean;
  mobileMenuVisible: boolean;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  toggleMobileMenu: () => void;
  toggleThemeColor: (color: string) => void;
  setThemeMode: (mode: ThemeMode) => void;
}

export type ThemeMode = 'system' | 'light' | 'dark';

export interface ITheme {
  mode: ThemeMode;
  color?: string;
}

export interface ISidebarStatus {
  minimized: boolean;
}

export const LayoutContext = createContext<IContext | undefined>(undefined);
LayoutContext.displayName = 'LayoutContext';

export const useLayoutContext = () => useRequiredContext(LayoutContext);
