import { useContextSelector } from 'use-context-selector';
import GhcrImageTable from './GhcrImageTable';
import SelectRegistryInput from './SelectRegistryInput';
import { ImagesContext } from './ImagesProvider';
import Loader from '@/components/ui/loader';
import { AlertMessage } from '@/components/ui/alert-message';
import { useNavigate } from 'react-router';
import { RegistryDiscriminator } from '@/api/_generated';

export default function ExternalImages() {
  const navigate = useNavigate();
  const registries = useContextSelector(ImagesContext, (v) => v?.registries) ?? [];
  const isLoading = useContextSelector(ImagesContext, (v) => v?.isLoading) ?? false;
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);

  if (isLoading) return <Loader />;
  if (!isLoading && registries?.length === 0)
    return (
      <AlertMessage type="info">
        <span>No registry has been configured yet, please add a new registry</span>
        <button className="font-semibold underline hover:no-underline ml-1" onClick={() => navigate('/registries/add')}>
          here
        </button>
        .
      </AlertMessage>
    );
  return (
    <>
      <SelectRegistryInput />
      {selectedRegistry?.discriminator === RegistryDiscriminator.GitHub && (
        <GhcrImageTable registryName={selectedRegistry.name!} />
      )}
    </>
  );
}
