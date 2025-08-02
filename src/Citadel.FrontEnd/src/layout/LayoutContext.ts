import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  theme: ITheme;
  sidebarMinimized: boolean;
  mobileMenuVisible: boolean;
  toggleSidebar: () => void;
  toggleMobileMenu: () => void;
  toggleThemeColor: (color: string) => void;
  setThemeMode: (mode: 'light' | 'dark') => void;
}

export interface ITheme {
  mode: 'light' | 'dark';
  color?: string;
}

export const LayoutContext = createContext<IContext | undefined>(undefined);
LayoutContext.displayName = 'LayoutContext';

export const useLayoutContext = () => useRequiredContext(LayoutContext);
