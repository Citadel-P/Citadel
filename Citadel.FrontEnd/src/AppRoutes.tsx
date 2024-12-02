import { createBrowserRouter, LoaderFunctionArgs, redirect, RouterProvider } from 'react-router';
import Layout from './layout/Layout';
import NotFound from './pages/NotFound';
import { useAuthContext } from './features/login/AuthProvider';

export const paths = [
  '/',
  'add-docker-platform',
  'platforms/:platformId/containers',
  'containers/:containerId/logs',
  'containers/:containerId/stats',
  'login',
  '*',
];

export const AppRoutes = () => {
  const { isAuthenticated } = useAuthContext();
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
          path: '*',
          element: <NotFound />,
        },
      ],
    },
    {
      path: paths[5],
      loader: loginLoader,
      lazy: async () => {
        return { Component: (await import('./features/login/Login')).default };
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
