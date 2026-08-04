import { BrowserRouter, Routes, Route } from 'react-router';
import Layout from '@/layout/layout';
import NotFound from '@/pages/not-found';
import Loader from './components/ui/loader';
import { lazy, Suspense } from 'react';
import { ResourceForm } from './pages/resource-form';
import { RequireAuth, RequireNoAuth } from './features/auth/auth-route-guards';
import { RequireSetup, RequireSetupComplete } from './features/setup/setup-route-guards';

export { REDIRECT_TO_KEY } from './features/auth/auth-route-guards';

const Login = lazy(() => import('@/features/auth/login'));
const MfaVerify = lazy(() => import('@/features/auth/mfa/verify'));
const MandatoryMfaSetup = lazy(() => import('@/features/auth/mfa/mandatory-setup'));
const Resources = lazy(() => import('@/pages/resource'));
const ResourceInfo = lazy(() => import('@/pages/resource-docker-info'));
const Profile = lazy(() => import('@/features/profile'));
const License = lazy(() => import('@/features/license'));
const InitialSetup = lazy(() => import('@/features/setup'));
const SwarmResource = lazy(() => import('@/features/swarm'));

export const Router = () => {
  return (
    <Suspense fallback={<Loader />}>
      <BrowserRouter>
        <Routes>
          <Route element={<RequireSetup />}>
            <Route path="setup" element={<InitialSetup />} />
          </Route>

          <Route element={<RequireSetupComplete />}>
            <Route element={<RequireNoAuth />}>
              <Route path="login" element={<Login />} />
              <Route path="login/mfa" element={<MfaVerify />} />
              <Route path="login/mfa/setup" element={<MandatoryMfaSetup />} />
            </Route>

            <Route element={<RequireAuth />}>
              <Route path="/" element={<Layout />}>
                <Route index element={<Resources />} />
                <Route path="profile" element={<Profile />} />
                <Route path="license" element={<License />} />
                <Route path=":type/edit/:id" element={<ResourceForm mode="edit" />} />
                <Route path=":type/:tab/edit/:id" element={<ResourceForm mode="edit" />} />

                <Route path="platforms">
                  <Route index element={<Resources />} />
                  <Route path=":platformId/swarm/:resourceType/:resourceId?" element={<SwarmResource />} />
                  <Route path=":platformId/:type" element={<Resources />} />
                  <Route path=":platformId/:type/add" element={<ResourceForm mode="add" />} />
                  <Route path=":platformId/:type/:resourceId" element={<ResourceInfo />} />
                </Route>

                <Route path=":type" element={<Resources />} />
                <Route path=":type/:tab" element={<Resources />} />
                <Route path=":type/add" element={<ResourceForm mode="add" />} />
                <Route path=":type/:tab/add" element={<ResourceForm mode="add" />} />

                <Route path="*" element={<NotFound />} />
              </Route>
            </Route>
          </Route>
        </Routes>
      </BrowserRouter>
    </Suspense>
  );
};
