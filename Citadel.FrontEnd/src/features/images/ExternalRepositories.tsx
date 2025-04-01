import { useContextSelector } from 'use-context-selector';
import GhcrImagesTable from './GhcrImagesTable';
import SelectRegistryInput from './SelectRegistryInput';
import { ImagesContext } from './ImagesProvider';
import Loader from '@/components/ui/loader';
import { RegistryDiscriminator } from '@/api/_generated';
import PrivateDockerHubImagesTable from './DockerHubImagesTable';
import { PublicDockerHubImages } from './PublicDockerHubImages';

export default function ExternalRepositories() {
  const isLoading = useContextSelector(ImagesContext, (v) => v?.isLoading) ?? false;
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);

  if (isLoading) return <Loader />;

  return (
    <>
      <SelectRegistryInput />
      {renderRegistryContent(selectedRegistry)}
    </>
  );
}

// Helper function to render content based on the selected registry
function renderRegistryContent(selectedRegistry: any) {
  if (!selectedRegistry) {
    return (
      <div className="text-center text-slate-600 dark:text-slate-400 mt-4">
        <p>Please select a registry to view its repositories.</p>
      </div>
    );
  }

  switch (selectedRegistry.discriminator) {
    case RegistryDiscriminator.GitHub:
      return <GhcrImagesTable registryName={selectedRegistry.name!} />;
    case RegistryDiscriminator.DockerHub:
      return selectedRegistry.isDefault ? (
        <PublicDockerHubImages />
      ) : (
        <PrivateDockerHubImagesTable registryName={selectedRegistry.name!} />
      );
    default:
      return (
        <div className="text-center text-red-500 mt-4">
          <p>Unsupported registry type.</p>
        </div>
      );
  }
}
