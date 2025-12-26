import { Navigate, Outlet, useLocation, BrowserRouter, Routes, Route } from 'react-router';
import Layout from '@/layout/layout';
import NotFound from '@/pages/not-found';
import { useAuthContext } from './features/auth/auth-context';
import Loader from './components/ui/loader';
import { lazy, Suspense } from 'react';
import { ResourceForm } from './pages/resource-form';

const Login = lazy(() => import('@/features/auth/login'));
const Resources = lazy(() => import('@/pages/resource'));
const ResourceInfo = lazy(() => import('@/pages/resource-docker-info'));

export const REDIRECT_TO_KEY = 'redirectTo';

export const Router = () => {
  return (
    <Suspense fallback={<Loader />}>
      <BrowserRouter>
        <Routes>
          <Route element={<RequireNoAuth />}>
            <Route path="login" element={<Login />} />
          </Route>

          <Route element={<RequireAuth />}>
            <Route path="/" element={<Layout />}>
              <Route index element={<Resources />} />

              <Route path="platforms">
                <Route index element={<Resources />} />
                <Route path=":platformId/:type" element={<Resources />} />
                <Route path=":platformId/:type/add" element={<ResourceForm mode="add" />} />
                <Route path=":platformId/:type/:resourceId" element={<ResourceInfo />} />
              </Route>

              <Route path=":type" element={<Resources />} />
              <Route path=":type/add" element={<ResourceForm mode="add" />} />
              <Route path=":type/edit/:id" element={<ResourceForm mode="edit" />} />

              <Route path="*" element={<NotFound />} />
            </Route>
          </Route>
        </Routes>
      </BrowserRouter>
    </Suspense>
  );
};

const RequireAuth = () => {
  const { isAuthenticated, isAuthReady, accessToken } = useAuthContext();
  const location = useLocation();

  if (!isAuthReady) {
    return <Loader />;
  }

  if (!isAuthenticated || !accessToken) {
    const currentUrl = `${location.pathname}${location.search}${location.hash}`;
    sessionStorage.setItem(REDIRECT_TO_KEY, currentUrl || '/');
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
};

const RequireNoAuth = () => {
  const { isAuthenticated, isAuthReady } = useAuthContext();

  if (!isAuthReady) {
    return <Loader />;
  }

  if (isAuthenticated) {
    const redirectTo = sessionStorage.getItem(REDIRECT_TO_KEY) ?? '/';
    return <Navigate to={redirectTo} replace />;
  }

  return <Outlet />;
};
