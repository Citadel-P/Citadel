import PlatformForm from '@/features/platforms/forms/PlatformForm';
import PlatformFormProvider from '@/features/platforms/forms/PlatformFormProvider';

export default function PlatformFormPage() {
  return (
    <PlatformFormProvider>
      <PlatformForm />
    </PlatformFormProvider>
  );
}
