import RegistryForm from '@/features/registries/forms/RegistryForm';
import Registries from '@/features/registries/Registries';
import { RegistryFormProvider } from '@/features/registries/forms/RegistryFormProvider';
import { RegistriesProvider } from '@/features/registries/RegistriesProvider';

const RegistriesPage = () => {
  return (
    <RegistriesProvider>
      <Registries />
    </RegistriesProvider>
  );
};

export const RegistryFormPage = () => {
  return (
    <RegistryFormProvider>
      <RegistryForm />
    </RegistryFormProvider>
  );
};

export default RegistriesPage;
