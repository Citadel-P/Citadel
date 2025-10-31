import Registries from '@/features/registries/Registries';
import { RegistriesProvider } from '@/features/registries/RegistriesProvider';

export default function RegistriesPage() {
  return (
    <RegistriesProvider>
      <Registries />
    </RegistriesProvider>
  );
}
