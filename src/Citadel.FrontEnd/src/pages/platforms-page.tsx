import Platforms from '@/features/platforms/platforms';
import { PlatformsProvider } from '@/features/platforms/PlatformsProvider';

export default function PlatformPage() {
  return (
    <PlatformsProvider>
      <Platforms />
    </PlatformsProvider>
  );
}
