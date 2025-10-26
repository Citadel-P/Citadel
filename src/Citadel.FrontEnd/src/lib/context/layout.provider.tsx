import { useState, useEffect, useMemo, useLayoutEffect } from 'react';
import { ITheme, LayoutContext } from './layout-context';

const useIsoLayoutEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;

const getInitialTheme = (): ITheme => {
  if (typeof window !== 'undefined') {
    const stored = window.localStorage.getItem('theme');
    if (stored) return JSON.parse(stored) as ITheme;
  }
  return { color: 'blue', mode: 'light' };
};

const getInitialSidebarMinimized = (): boolean => {
  if (typeof window !== 'undefined') {
    const stored = window.localStorage.getItem('sidebarStatus');
    if (stored) return (JSON.parse(stored) as { minimized: boolean }).minimized;
  }
  return false;
};

export const LayoutProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState<ITheme>(getInitialTheme);
  const [mobileMenuVisible, setMobileMenuVisibility] = useState(false);
  const [sidebarMinimized, setSidebarMinimized] = useState<boolean>(getInitialSidebarMinimized);

  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth < 1024) {
        setSidebarMinimized(true);
      }
    };

    window.addEventListener('resize', handleResize);
    if (window.innerWidth < 1024) {
      setSidebarMinimized(true);
    }

    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useIsoLayoutEffect(() => {
    const html = document.documentElement;
    if (html) {
      html.setAttribute('data-theme', theme.color ?? 'base');
      html.classList.remove('light', 'dark');
      html.classList.add(theme.mode);
    }
  }, [theme]);

  useEffect(() => {
    window.localStorage.setItem('theme', JSON.stringify(theme));
    window.localStorage.setItem('sidebarStatus', JSON.stringify({ minimized: sidebarMinimized }));
  }, [theme, sidebarMinimized]);

  const value = useMemo(
    () => ({
      theme,
      sidebarMinimized,
      mobileMenuVisible,
      toggleSidebar: (): void => setSidebarMinimized((prev) => !prev),
      toggleMobileMenu: () => setMobileMenuVisibility((prev) => !prev),
      toggleThemeColor: (color: string): void => setTheme((prev: ITheme) => ({ ...prev, color })),
      setThemeMode: (mode: 'light' | 'dark'): void => setTheme((prev: ITheme) => ({ ...prev, mode })),
    }),
    [theme, sidebarMinimized, mobileMenuVisible],
  );

  return <LayoutContext.Provider value={value}>{children}</LayoutContext.Provider>;
};
