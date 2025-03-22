import { Outlet } from 'react-router';
import LayoutProvider, { LayoutContext } from './LayoutProvider';
import { Sidebar } from './sidebar/Sidebar';
import Breadcrumb from './breadcrumb/Breadcrumb';
import AppProvider from '@/AppProvider';
import { useContextSelector } from 'use-context-selector';

const LayoutPage = () => {
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized);

  return (
    <div className={`${sidebarMinimized ? 'sidebar-collapsed' : 'sidebar-expanded'} flex h-dvh w-full overflow-hidden`}>
      <Sidebar />
      <main className="flex grow flex-col content-start overflow-hidden bg-card">
        <div className="scrollbar-thumb-rounded scrollbar-track-rounded grow overflow-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
          <Breadcrumb />
          <Outlet />
        </div>
      </main>
    </div>
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
