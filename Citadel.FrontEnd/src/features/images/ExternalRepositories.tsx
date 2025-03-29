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
      {selectedRegistry?.discriminator === RegistryDiscriminator.GitHub && (
        <GhcrImagesTable registryName={selectedRegistry.name!} />
      )}
      {selectedRegistry?.discriminator === RegistryDiscriminator.DockerHub &&
        (selectedRegistry.isDefault ? (
          <PublicDockerHubImages />
        ) : (
          <PrivateDockerHubImagesTable registryName={selectedRegistry.name!} />
        ))}
    </>
  );
}
