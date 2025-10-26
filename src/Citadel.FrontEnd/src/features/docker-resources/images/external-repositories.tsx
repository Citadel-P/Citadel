import GhcrImagesTable from './ghcr-images';
import { RegistryType, RegistryView } from '@/api/generated/api.types';
import PrivateDockerHubImagesTable from './dockerhub-private-images';
import { PublicDockerHubImages } from './dockerhub-public-images';
import { useResourceFilter } from '@/lib/atoms';

export const ExternalRepositories = () => {
  const [filter] = useResourceFilter<{ item: RegistryView }>('Registry');
  const selectedRegistry = filter?.item;
  if (!selectedRegistry) {
    return (
      <div className="text-center text-slate-600 dark:text-slate-400 p-5">
        <p>Select a registry from the dropdown to browse its repositories.</p>
      </div>
    );
  }
  switch (selectedRegistry.type) {
    case RegistryType.GitHub:
      return <GhcrImagesTable registryName={selectedRegistry.name!} />;
    case RegistryType.DockerHub:
      return selectedRegistry.isDefault ? (
        <PublicDockerHubImages />
      ) : (
        <PrivateDockerHubImagesTable registryName={selectedRegistry.name!} />
      );
    default:
      return (
        <div className="text-center text-red-500 mt-4">
          <p>Registry type not supported.</p>
        </div>
      );
  }
};
