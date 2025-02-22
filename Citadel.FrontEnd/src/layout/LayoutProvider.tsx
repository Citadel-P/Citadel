import { useState } from 'react';
import { createContext } from 'use-context-selector';

interface IContext {
  theme: ITheme;
  sidebarMinimized: boolean;
  mobileMenuVisible: boolean;
  toggleSidebar: () => void;
  toggleMobileMenu: () => void;
  toggleThemeColor: (color: string) => void;
  setThemeMode: (mode: 'light' | 'dark') => void;
}

interface ITheme {
  mode: 'light' | 'dark';
  color?: string;
}

interface IProps {
  children?: React.ReactNode;
}

export const LayoutContext = createContext<IContext | undefined>(undefined);

const theme = localStorage.getItem('theme');
const initTheme: ITheme = theme ? JSON.parse(theme) : { color: 'blue', mode: 'light' };

const LayoutProvider: React.FC<IProps> = ({ children }) => {
  const [theme, setTheme] = useState(initTheme);
  const [mobileMenuVisible, setMobileMenuVisibility] = useState(false);
  const [sidebarMinimized, setSidebarMinimized] = useState(false);

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

export default LayoutProvider;
