import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  sidebarMinimized: boolean;
  mobileMenuVisible: boolean;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  toggleMobileMenu: () => void;
}

export const LayoutContext = createContext<IContext | undefined>(undefined);
LayoutContext.displayName = 'LayoutContext';

export const useLayoutContext = () => useRequiredContext(LayoutContext);
