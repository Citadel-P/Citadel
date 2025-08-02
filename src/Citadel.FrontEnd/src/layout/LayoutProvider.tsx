import { useState, useEffect } from 'react';
import { ITheme, LayoutContext } from './LayoutContext';

const theme = localStorage.getItem('theme');
const initTheme: ITheme = theme ? JSON.parse(theme) : { color: 'blue', mode: 'light' };

export const LayoutProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState(initTheme);
  const [mobileMenuVisible, setMobileMenuVisibility] = useState(false);
  const [sidebarMinimized, setSidebarMinimized] = useState(false);

  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth < 1024) {
        setSidebarMinimized(true);
      } else {
        setSidebarMinimized(false);
      }
    };

    window.addEventListener('resize', handleResize);
    handleResize(); // Initial check

    return () => window.removeEventListener('resize', handleResize);
  }, []);

  document.querySelector('html')!.setAttribute('data-theme', theme.color ?? 'base');
  document.querySelector('html')!.className = theme.mode;
  localStorage.setItem('theme', JSON.stringify(theme));

  return (
    <LayoutContext.Provider
      value={{
        theme,
        sidebarMinimized,
        mobileMenuVisible,
        toggleSidebar: (): void => setSidebarMinimized(!sidebarMinimized),
        toggleMobileMenu: () => setMobileMenuVisibility(!mobileMenuVisible),
        toggleThemeColor: (color: string): void => setTheme((prevState) => ({ ...prevState, color })),
        setThemeMode: (mode: 'light' | 'dark'): void => setTheme((prevState) => ({ ...prevState, mode })),
      }}>
      {children}
    </LayoutContext.Provider>
  );
};
