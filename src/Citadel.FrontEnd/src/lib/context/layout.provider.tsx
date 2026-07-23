import { useState, useEffect, useMemo, useLayoutEffect, useCallback } from 'react';
import { ITheme, LayoutContext, ThemeMode } from './layout-context';

const useIsoLayoutEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;
const prefersDarkQuery = '(prefers-color-scheme: dark)';

const getSystemThemeMode = (): 'light' | 'dark' => {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia(prefersDarkQuery).matches ? 'dark' : 'light';
};

const normalizeTheme = (theme: ITheme): ITheme => {
  const mode = theme.mode === 'system' || theme.mode === 'dark' || theme.mode === 'light' ? theme.mode : 'light';
  return { color: theme.color ?? 'blue', mode };
};

const getInitialTheme = (): ITheme => {
  if (typeof window !== 'undefined') {
    const stored = window.localStorage.getItem('theme');
    if (stored) {
      try {
        return normalizeTheme(JSON.parse(stored) as ITheme);
      } catch {
        window.localStorage.removeItem('theme');
      }
    }
  }
  return { color: 'blue', mode: 'light' };
};

// 👇 Now accounts for screen width when no stored preference exists
const getInitialSidebarMinimized = (): boolean => {
  if (typeof window !== 'undefined') {
    const stored = window.localStorage.getItem('sidebarStatus');
    if (stored) {
      return (JSON.parse(stored) as { minimized: boolean }).minimized;
    }
    // No stored preference – respect current screen size
    return window.innerWidth < 1024;
  }
  return false;
};

export const LayoutProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState<ITheme>(getInitialTheme);
  const [systemMode, setSystemMode] = useState<'light' | 'dark'>(getSystemThemeMode);
  const [mobileMenuVisible, setMobileMenuVisibility] = useState(false);
  const [sidebarMinimized, setSidebarMinimized] = useState<boolean>(getInitialSidebarMinimized);

  const toggleSidebar = useCallback((): void => setSidebarMinimized((prev) => !prev), []);
  const setSidebarOpen = useCallback((open: boolean): void => setSidebarMinimized(!open), []);
  const toggleMobileMenu = useCallback(() => setMobileMenuVisibility((prev) => !prev), []);
  const toggleThemeColor = useCallback(
    (color: string): void => setTheme((prev: ITheme) => (prev.color === color ? prev : { ...prev, color })),
    [],
  );
  const setThemeMode = useCallback(
    (mode: ThemeMode): void => setTheme((prev: ITheme) => (prev.mode === mode ? prev : { ...prev, mode })),
    [],
  );

  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth < 1024 && !sidebarMinimized) {
        setSidebarMinimized(true);
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [sidebarMinimized]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const media = window.matchMedia(prefersDarkQuery);
    const handleChange = () => setSystemMode(media.matches ? 'dark' : 'light');
    handleChange();
    media.addEventListener('change', handleChange);
    return () => media.removeEventListener('change', handleChange);
  }, []);

  useIsoLayoutEffect(() => {
    const html = document.documentElement;
    const effectiveMode = theme.mode === 'system' ? systemMode : theme.mode;
    if (html) {
      html.setAttribute('data-theme', theme.color ?? 'base');
      html.classList.remove('light', 'dark');
      html.classList.add(effectiveMode);
    }
  }, [theme, systemMode]);

  useEffect(() => {
    window.localStorage.setItem('theme', JSON.stringify(theme));
    window.localStorage.setItem('sidebarStatus', JSON.stringify({ minimized: sidebarMinimized }));
  }, [theme, sidebarMinimized]);

  const value = useMemo(
    () => ({
      theme,
      sidebarMinimized,
      mobileMenuVisible,
      toggleSidebar,
      setSidebarOpen,
      toggleMobileMenu,
      toggleThemeColor,
      setThemeMode,
    }),
    [
      theme,
      sidebarMinimized,
      mobileMenuVisible,
      toggleSidebar,
      setSidebarOpen,
      toggleMobileMenu,
      toggleThemeColor,
      setThemeMode,
    ],
  );

  return <LayoutContext.Provider value={value}>{children}</LayoutContext.Provider>;
};
