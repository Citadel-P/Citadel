import GhcrImagesTable from './GhcrImagesTable';
import SelectRegistryInput from './SelectRegistryInput';
import { useImagesContext } from './ImagesContext';
import Loader from '@/components/ui/loader';
import { RegistryType } from '@/api/generated/api.types';
import PrivateDockerHubImagesTable from './DockerHubImagesTable';
import { PublicDockerHubImages } from './PublicDockerHubImages';

export default function ExternalRepositories() {
  const { selectedRegistry, isLoading } = useImagesContext();

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
        <p>Please choose a registry to display its repositories.</p>
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
}
