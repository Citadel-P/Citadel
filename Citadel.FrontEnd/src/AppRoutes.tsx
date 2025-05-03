import { createBrowserRouter, redirect, RouterProvider } from 'react-router';
import Layout from '@/layout/Layout';
import NotFound from '@/pages/NotFound';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

export const AppPaths: Record<string, string> = {
  any: '*',
  login: 'login',
  main: '/',
  addDockerPlatform: 'add-docker-platform',
  platformContainers: 'platforms/:platformId/containers',
  containerLogs: 'containers/:containerId/logs',
  containerStats: 'containers/:containerId/stats',
  containerInspect: 'containers/:containerId/inspect',
  registries: 'registries',
  addRegistry: 'registries/add',
  editRegistry: 'registries/edit/:registryId',
  images: 'platforms/:platformId/images',
  localImages: 'platforms/:platformId/images/local',
  externalImages: 'platforms/:platformId/images/external',
  networks: 'platforms/:platformId/networks',
  addNetwork: 'platforms/:platformId/networks/add',
};

export const AppRoutes = () => {
  const isAuthenticated = useContextSelector(AuthContext, (v) => v?.isAuthenticated);

  const protectedRoutes = [
    {
      path: AppPaths.main,
      element: <Layout />,
      loader: () => {
        return !isAuthenticated ? redirect(AppPaths.login) : null;
      },
      hydrateFallbackElement: <Fallback />,
      children: [
        {
          index: true,
          lazy: async () => {
            return { Component: (await import('@/pages/platforms-page')).default };
          },
        },
        {
          path: AppPaths.addDockerPlatform,
          lazy: async () => {
            return { Component: (await import('@/features/platforms/addDockerPlatform/AddDockerPltaform')).default };
          },
        },
        {
          path: AppPaths.platformContainers,
          lazy: async () => {
            return { Component: (await import('@/pages/containers-page')).default };
          },
        },
        {
          path: AppPaths.containerLogs,
          lazy: async () => {
            return { Component: (await import('@/features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.containerStats,
          lazy: async () => {
            return { Component: (await import('@/features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.containerInspect,
          lazy: async () => {
            return { Component: (await import('@/features/containers/container-info/ContainerInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.registries,
          lazy: async () => {
            return { Component: (await import('@/pages/registries-page')).default };
          },
        },
        {
          path: AppPaths.addRegistry,
          lazy: async () => {
            return { Component: (await import('@/pages/registries-page')).RegistryFormPage };
          },
        },
        {
          path: AppPaths.editRegistry,
          lazy: async () => {
            return { Component: (await import('@/pages/registries-page')).RegistryFormPage };
          },
        },
        {
          path: AppPaths.images,
          lazy: async () => {
            return { Component: (await import('@/pages/images-page')).default };
          },
        },
        {
          path: AppPaths.localImages,
          lazy: async () => {
            return { Component: (await import('@/pages/images-page')).default };
          },
        },
        {
          path: AppPaths.externalImages,
          lazy: async () => {
            return { Component: (await import('@/pages/images-page')).default };
          },
        },
        {
          path: AppPaths.networks,
          lazy: async () => {
            return { Component: (await import('@/pages/networks-page')).default };
          },
        },
        {
          path: AppPaths.addNetwork,
          lazy: async () => {
            return { Component: (await import('@/features/networks/forms/AddNetworkForm')).default };
          },
        },
        { path: AppPaths.any, element: <NotFound /> },
      ],
    },
  ];

  const publicRoutes = [
    {
      path: AppPaths.login,
      loader: () => {
        return isAuthenticated ? redirect(AppPaths.main) : null;
      },
      lazy: async () => {
        return { Component: (await import('@/features/auth/Login')).default };
      },
    },
  ];

  // Define the router
  const router = createBrowserRouter([...publicRoutes, ...protectedRoutes]);

  // Dispose router on hot reload
  if (import.meta.hot) {
    import.meta.hot.dispose(() => router.dispose());
  }

  return <RouterProvider router={router} />;
};

// Fallback component for lazy loading
function Fallback() {
  return <p>Loading...</p>;
}
