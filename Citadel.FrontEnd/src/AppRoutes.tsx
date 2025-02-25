import { createBrowserRouter, LoaderFunctionArgs, redirect, RouterProvider } from 'react-router';
import Layout from './layout/Layout';
import NotFound from './pages/NotFound';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

export const paths = [
  '/',
  'add-docker-platform',
  'platforms/:platformId/containers',
  'containers/:containerId/logs',
  'containers/:containerId/stats',
  'containers/:containerId/inspect',
  'login',
  '*',
];

export const AppRoutes = () => {
  const isAuthenticated = useContextSelector(AuthContext, (v) => v?.isAuthenticated);
  const router = createBrowserRouter([
    {
      path: paths[0],
      element: <Layout />,
      loader: protectedLoader,
      hydrateFallbackElement: <Fallback />,
      children: [
        {
          index: true,
          lazy: async () => {
            return { Component: (await import('./pages/platforms-page')).default };
          },
        },
        {
          path: paths[1],
          lazy: async () => {
            return { Component: (await import('./features/platforms/addDockerPlatform/AddDockerPltaform')).default };
          },
        },
        {
          path: paths[2],
          lazy: async () => {
            return { Component: (await import('./pages/containers-page')).default };
          },
        },
        {
          path: paths[3],
          lazy: async () => {
            return { Component: (await import('./pages/container-info-page')).default };
          },
        },
        {
          path: paths[4],
          lazy: async () => {
            return { Component: (await import('./pages/container-info-page')).default };
          },
        },
        {
          path: paths[5],
          lazy: async () => {
            return { Component: (await import('./pages/container-info-page')).default };
          },
        },
        {
          path: '*',
          element: <NotFound />,
        },
      ],
    },
    {
      path: paths[6],
      loader: loginLoader,
      lazy: async () => {
        return { Component: (await import('./features/auth/Login')).default };
      },
    },
  ]);

  function protectedLoader({ request }: LoaderFunctionArgs) {
    if (!isAuthenticated) {
      return redirect('/login');
    }
    return null;
  }

  function loginLoader({ request }: LoaderFunctionArgs) {
    if (isAuthenticated) {
      return redirect('/');
    }
    return null;
  }

  if (import.meta.hot) {
    import.meta.hot.dispose(() => router.dispose());
  }

  return <RouterProvider router={router} />;
};

function Fallback() {
  return <p>Loading...</p>;
}
