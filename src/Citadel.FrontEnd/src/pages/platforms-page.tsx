import Platforms from '@/features/platforms/Platforms';
import { PlatformsProvider } from '@/features/platforms/PlatformsProvider';

export default function PlatformPage() {
  return (
    <PlatformsProvider>
      <Platforms />
    </PlatformsProvider>
  );
}
