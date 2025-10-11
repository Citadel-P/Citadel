import { createBrowserRouter, redirect, RouterProvider } from 'react-router';
import Layout from '@/layout/Layout';
import NotFound from '@/pages/NotFound';
import { useAuthContext } from './features/auth/AuthContext';

export const AppPaths: Record<string, string> = {
  any: '*',
  login: 'login',
  main: '/',
  platforms: 'platforms',
  addPlatform: 'platforms/add',
  platformContainers: 'platforms/:platformId/containers',
  container: 'containers/:containerId',
  containerLogs: 'containers/:containerId/logs',
  containerStats: 'containers/:containerId/stats',
  containerInspect: 'containers/:containerId/inspect',
  registries: 'registries',
  addRegistry: 'registries/add',
  editRegistry: 'registries/edit/:registryId',
  images: 'platforms/:platformId/images',
  image: 'platforms/:platformId/images/:resourceId',
  imageInspect: 'platforms/:platformId/images/:resourceId/inspect',
  localImages: 'platforms/:platformId/images/local',
  externalImages: 'platforms/:platformId/images/external',
  networks: 'platforms/:platformId/networks',
  network: 'platforms/:platformId/networks/:resourceId',
  networkInspect: 'platforms/:platformId/networks/:resourceId/inspect',
  addNetwork: 'platforms/:platformId/networks/add',
  volumes: 'platforms/:platformId/volumes',
  volume: 'platforms/:platformId/volumes/:resourceId',
  volumeInspect: 'platforms/:platformId/volumes/:resourceId/inspect',
  addVolume: 'platforms/:platformId/volumes/add',
  deployments: 'deployments',
  addDeployment: 'deployments/add',
  editDeployment: 'deployments/edit/:deploymentId',
};

export const AppRoutes = () => {
  const { isAuthenticated } = useAuthContext();
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
          path: AppPaths.platformContainers,
          lazy: async () => {
            return { Component: (await import('@/pages/containers-page')).default };
          },
        },
        {
          path: AppPaths.container,
          lazy: async () => {
            return { Component: (await import('@/features/containers/container-info/ContainerInfoWrapper')).default };
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
          path: AppPaths.image,
          lazy: async () => {
            return { Component: (await import('@/features/images/image-info/ImageInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.imageInspect,
          lazy: async () => {
            return { Component: (await import('@/features/images/image-info/ImageInfoWrapper')).default };
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
          path: AppPaths.network,
          lazy: async () => {
            return { Component: (await import('@/features/networks/network-info/NetworkInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.networkInspect,
          lazy: async () => {
            return { Component: (await import('@/features/networks/network-info/NetworkInfoWrapper')).default };
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
        {
          path: AppPaths.volume,
          lazy: async () => {
            return { Component: (await import('@/features/volumes/volume-info/VolumeInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.volumeInspect,
          lazy: async () => {
            return { Component: (await import('@/features/volumes/volume-info/VolumeInfoWrapper')).default };
          },
        },
        {
          path: AppPaths.volumes,
          lazy: async () => {
            return { Component: (await import('@/pages/volumes-page')).default };
          },
        },
        {
          path: AppPaths.addVolume,
          lazy: async () => {
            return { Component: (await import('@/features/volumes/forms/AddVolumeForm')).default };
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
