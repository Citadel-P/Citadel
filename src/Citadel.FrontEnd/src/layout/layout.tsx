import { Outlet } from 'react-router';
import { useLayoutContext } from '../lib/context/layout-context';
import { LayoutProvider } from '../lib/context/layout.provider';
import { AppProvider } from '@/lib/context/app-provider';
import { useEffect } from 'react';
import { Sidebar } from './sidebar/sidebar';
import { useRead } from '@/lib/hooks';
import { toThemeMode } from '@/lib/theme-preferences';
import { LicenseReminder } from './license-reminder';
import { Header } from './header';
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar';

const LayoutPage = () => {
  const { sidebarMinimized, setSidebarOpen, setThemeMode } = useLayoutContext();
  const preferencesQuery = useRead('getProfilePreferences');
  const preferredTheme = preferencesQuery.data?.data.theme;

  useEffect(() => {
    if (preferredTheme) {
      setThemeMode(toThemeMode(preferredTheme));
    }
  }, [preferredTheme, setThemeMode]);

  return (
    <SidebarProvider
      open={!sidebarMinimized}
      onOpenChange={setSidebarOpen}
      className={`${sidebarMinimized ? 'sidebar-collapsed' : 'sidebar-expanded'} h-dvh overflow-hidden`}>
      <Sidebar />
      <SidebarInset className="overflow-hidden bg-card">
        <Header />
        <div id="main-scroll-container" className="grow overflow-auto">
          <LicenseReminder />
          <Outlet />
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
};

const Layout = () => {
  return (
    <AppProvider>
      <LayoutProvider>
        <LayoutPage />
      </LayoutProvider>
    </AppProvider>
  );
};

export default Layout;
