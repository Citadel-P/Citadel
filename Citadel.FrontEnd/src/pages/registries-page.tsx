import Registries from '@/features/registries/Registries';
import RegistriesProvider from '@/features/registries/RegistriesProvider';

const RegistriesPage = () => {
  return (
    <RegistriesProvider>
      <Registries />
    </RegistriesProvider>
  );
};

export default RegistriesPage;
