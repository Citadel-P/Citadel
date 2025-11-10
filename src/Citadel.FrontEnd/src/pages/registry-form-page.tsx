import RegistryForm from '@/features/registries/forms/registry-form';
import { RegistryFormProvider } from '@/features/registries/forms/registry-form-provider';

export default function RegistryFormPage() {
  return (
    <RegistryFormProvider>
      <RegistryForm />
    </RegistryFormProvider>
  );
}
