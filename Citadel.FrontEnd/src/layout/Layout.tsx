import { Outlet } from 'react-router-dom';
import LayoutProvider from './LayoutProvider';
import { Navbar } from './navbar/Navbar';
import { Sidebar } from './sidebar/Sidebar';
import Breadcrumb from './breadcrumb/Breadcrumb';
import AppProvider from '@/AppProvider';

const Layout = () => {
  return (
    <AppProvider>
      <LayoutProvider>
        <div className="flex h-screen w-full overflow-hidden">
          <Sidebar />
          <div className="flex grow flex-col content-start overflow-hidden bg-card">
            <Navbar />
            <div className="min-h-full scrollbar-thumb-rounded scrollbar-track-rounded scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
              <Breadcrumb />
              <Outlet />
            </div>
          </div>
        </div>
      </LayoutProvider>
    </AppProvider>
  );
};

export default Layout;
