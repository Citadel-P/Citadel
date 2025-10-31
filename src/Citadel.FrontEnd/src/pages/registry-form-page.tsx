import RegistryForm from '@/features/registries/forms/RegistryForm';
import { RegistryFormProvider } from '@/features/registries/forms/RegistryFormProvider';

export default function RegistryFormPage() {
  return (
    <RegistryFormProvider>
      <RegistryForm />
    </RegistryFormProvider>
  );
}
