import PlatformForm from '@/features/platforms/forms/PlatformForm';
import PlatformFormProvider from '@/features/platforms/forms/PlatformFormProvider';
import Platforms from '@/features/platforms/Platforms';
import { PlatformsProvider } from '@/features/platforms/PlatformsProvider';

const PlatformPage = () => {
  return (
    <PlatformsProvider>
      <Platforms />
    </PlatformsProvider>
  );
};

export const PlatformFormPage = () => {
  return (
    <PlatformFormProvider>
      <PlatformForm />
    </PlatformFormProvider>
  );
};

export default PlatformPage;
