import { createBrowserRouter, redirect, RouterProvider } from 'react-router';
import Layout from '@/layout/Layout';
import NotFound from '@/pages/NotFound';
import { useAuthContext } from './features/auth/AuthContext';
import Loader from './components/ui/loader';

export const AppPaths: Record<string, string> = {
  any: '*',
  login: 'login',
  main: '/',
  platforms: 'platforms',
  addPlatform: 'platforms/add',
  container: 'containers/:containerId',
  containerLogs: 'containers/:containerId/logs',
  containerStats: 'containers/:containerId/stats',
  containerInspect: 'containers/:containerId/inspect',
  registries: 'registries',
  addRegistry: 'registries/add',
  editRegistry: 'registries/edit/:registryId',
  addNetwork: 'platforms/:platformId/networks/add',
  resource: 'platforms/:platformId/:type',
  resourceInfo: 'platforms/:platformId/:type/:resourceId',
  addVolume: 'platforms/:platformId/volumes/add',
  deployments: 'deployments',
  addDeployment: 'deployments/add',
  editDeployment: 'deployments/edit/:deploymentId',
};

export const REDIRECT_TO_KEY = 'redirectTo';

export const AppRoutes = () => {
  const { isAuthenticated } = useAuthContext();
  const protectedRoutes = [
    {
      path: AppPaths.main,
      element: <Layout />,
      loader: ({ request }: any) => {
        if (!isAuthenticated) {
          const currentUrl = new URL(request.url).pathname;
          sessionStorage.setItem(REDIRECT_TO_KEY, currentUrl);
          return redirect(AppPaths.login);
        }
        return null;
      },
      hydrateFallbackElement: <Loader />,
      children: [
        {
          index: true,
          lazy: async () => {
            return { Component: (await import('@/pages/platforms-page')).default };
          },
        },
        {
          path: AppPaths.platforms,
          lazy: async () => {
            return { Component: (await import('@/pages/platforms-page')).default };
          },
        },
        {
          path: AppPaths.addPlatform,
          lazy: async () => {
            return { Component: (await import('@/pages/platforms-page')).PlatformFormPage };
          },
        },
        {
          path: AppPaths.container,
          lazy: async () => {
            return {
              Component: (await import('@/features/docker-resources/containers/container-info/ContainerInfoWrapper'))
                .default,
            };
          },
        },
        {
          path: AppPaths.containerLogs,
          lazy: async () => {
            return {
              Component: (await import('@/features/docker-resources/containers/container-info/ContainerInfoWrapper'))
                .default,
            };
          },
        },
        {
          path: AppPaths.containerStats,
          lazy: async () => {
            return {
              Component: (await import('@/features/docker-resources/containers/container-info/ContainerInfoWrapper'))
                .default,
            };
          },
        },
        {
          path: AppPaths.containerInspect,
          lazy: async () => {
            return {
              Component: (await import('@/features/docker-resources/containers/container-info/ContainerInfoWrapper'))
                .default,
            };
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
          path: AppPaths.deployments,
          lazy: async () => {
            return { Component: (await import('@/pages/deployments-page')).default };
          },
        },
        {
          path: AppPaths.addDeployment,
          lazy: async () => {
            return { Component: (await import('@/pages/deployments-page')).DeploymentFormPage };
          },
        },
        {
          path: AppPaths.editDeployment,
          lazy: async () => {
            return { Component: (await import('@/pages/deployments-page')).DeploymentFormPage };
          },
        },
        {
          path: AppPaths.addNetwork,
          lazy: async () => {
            return { Component: (await import('@/features/docker-resources/networks/forms/add-network')).default };
          },
        },
        {
          path: AppPaths.resource,
          lazy: async () => {
            return { Component: (await import('@/pages/docker-resource')).default };
          },
        },
        {
          path: AppPaths.resourceInfo,
          lazy: async () => {
            return { Component: (await import('@/pages/docker-resource-info')).default };
          },
        },
        {
          path: AppPaths.addVolume,
          lazy: async () => {
            return { Component: (await import('@/features/docker-resources/volumes/forms/add-volume')).default };
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

  const router = createBrowserRouter([...publicRoutes, ...protectedRoutes]);

  if (import.meta.hot) {
    import.meta.hot.dispose(() => router.dispose());
  }

  return <RouterProvider router={router} />;
};
