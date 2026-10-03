import { useState, useEffect, useMemo, useCallback, type ReactNode } from 'react';
import { LayoutContext } from './layout-context';

function initialSidebarMinimized(): boolean {
  try {
    const stored = JSON.parse(localStorage.getItem('sidebarStatus') ?? 'null');
    if (typeof stored?.minimized === 'boolean') return stored.minimized;
  } catch {
    /* Use the viewport default for an invalid or unavailable cache. */
  }
  return window.innerWidth < 1024;
}
export function LayoutProvider({ children }: { children?: ReactNode }) {
  const [sidebarMinimized, setSidebarMinimized] = useState(initialSidebarMinimized);
  const [mobileMenuVisible, setMobileMenuVisible] = useState(false);
  const toggleSidebar = useCallback(() => setSidebarMinimized((value) => !value), []);
  const setSidebarOpen = useCallback((open: boolean) => setSidebarMinimized(!open), []);
  const toggleMobileMenu = useCallback(() => setMobileMenuVisible((value) => !value), []);
  useEffect(() => {
    const resize = () => {
      if (window.innerWidth < 1024) setSidebarMinimized(true);
    };
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  }, []);
  useEffect(() => {
    try {
      localStorage.setItem('sidebarStatus', JSON.stringify({ minimized: sidebarMinimized }));
    } catch {
      /* Optional cache. */
    }
  }, [sidebarMinimized]);
  const value = useMemo(
    () => ({ sidebarMinimized, mobileMenuVisible, toggleSidebar, setSidebarOpen, toggleMobileMenu }),
    [sidebarMinimized, mobileMenuVisible, toggleSidebar, setSidebarOpen, toggleMobileMenu],
  );
  return <LayoutContext.Provider value={value}>{children}</LayoutContext.Provider>;
}
