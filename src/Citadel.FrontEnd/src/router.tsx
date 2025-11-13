import { Navigate, Outlet, useLocation, BrowserRouter, Routes, Route } from 'react-router';
import Layout from '@/layout/layout';
import NotFound from '@/pages/not-found';
import { useAuthContext } from './features/auth/auth-context';
import Loader from './components/ui/loader';
import { lazy, Suspense } from 'react';
import { ResourceFormPage } from './pages/resource-form-page';

const Login = lazy(() => import('@/features/auth/login'));
const Resources = lazy(() => import('@/pages/resource'));
const ResourceInfo = lazy(() => import('@/pages/docker-resource-info'));
const AddPlatform = lazy(() => import('@/pages/add-platform-page'));
const AddNetwork = lazy(() => import('@/features/docker-resources/networks/forms/add-network'));
const AddVolume = lazy(() => import('@/features/docker-resources/volumes/forms/add-volume'));
const AddRegistry = lazy(() => import('@/pages/registry-form-page'));
const EditRegistry = lazy(() => import('@/pages/registry-form-page'));

export const REDIRECT_TO_KEY = 'redirectTo';

export const Router = () => {
  return (
    <Suspense fallback={<Loader />}>
      <BrowserRouter>
        <Routes>
          <Route path="login" element={<Login />} />

          <Route element={<RequireAuth />}>
            <Route path="/" element={<Layout />}>
              <Route index element={<Resources />} />

              <Route path="platforms">
                <Route index element={<Resources />} />
                <Route path="add" element={<AddPlatform />} />
                <Route path=":platformId/:type" element={<Resources />} />
                <Route path=":platformId/:type/:resourceId" element={<ResourceInfo />} />
              </Route>

              <Route path=":type" element={<Resources />} />
              <Route path=":type/add" element={<ResourceFormPage mode="add" />} />
              <Route path=":type/edit/:id" element={<ResourceFormPage mode="edit" />} />

              <Route path="platforms/:platformId/networks/add" element={<AddNetwork />} />
              <Route path="platforms/:platformId/volumes/add" element={<AddVolume />} />
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
