import { createBrowserRouter, LoaderFunctionArgs, redirect, RouterProvider } from 'react-router';
import Layout from './layout/Layout';
import NotFound from './pages/NotFound';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

export const paths = [
  '*',
  'login',
  '/',
  'add-docker-platform',
  'platforms/:platformId/containers',
  'containers/:containerId/logs',
  'containers/:containerId/stats',
  'containers/:containerId/inspect',
  'registries',
  'registries/add',
  'registries/edit/:registryId',
  'platforms/:platformId/images',
  'platforms/:platformId/images/local',
  'platforms/:platformId/images/external',
];

export const AppRoutes = () => {
  const isAuthenticated = useContextSelector(AuthContext, (v) => v?.isAuthenticated);
  const router = createBrowserRouter([
    {
      path: paths[1],
      loader: loginLoader,
      lazy: async () => {
        return { Component: (await import('./features/auth/Login')).default };
      },
    },
    {
      path: paths[2],
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
          path: paths[3],
          lazy: async () => {
            return { Component: (await import('./features/platforms/addDockerPlatform/AddDockerPltaform')).default };
          },
        },
        {
          path: paths[4],
          lazy: async () => {
            return { Component: (await import('./pages/containers-page')).default };
          },
        },
        {
          path: paths[5],
          lazy: async () => {
            return { Component: (await import('./features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: paths[6],
          lazy: async () => {
            return { Component: (await import('./features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: paths[7],
          lazy: async () => {
            return { Component: (await import('./features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: paths[8],
          lazy: async () => {
            return { Component: (await import('./pages/registries-page')).default };
          },
        },
        {
          path: paths[9],
          lazy: async () => {
            return { Component: (await import('./pages/registries-page')).RegistryFormPage };
          },
        },
        {
          path: paths[10],
          lazy: async () => {
            return { Component: (await import('./pages/registries-page')).RegistryFormPage };
          },
        },
        {
          path: paths[11],
          lazy: async () => {
            return { Component: (await import('./pages/images-page')).default };
          },
        },
        {
          path: paths[12],
          lazy: async () => {
            return { Component: (await import('./pages/images-page')).default };
          },
        },
        {
          path: paths[13],
          lazy: async () => {
            return { Component: (await import('./pages/images-page')).default };
          },
        },
        {
          path: '*',
          element: <NotFound />,
        },
      ],
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
