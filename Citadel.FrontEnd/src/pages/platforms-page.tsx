import Platforms from '@/features/platforms/Platforms';
import PlatformsProvider from '@/features/platforms/PlatformsProvider';

const PlatformsPage = () => {
  return (
    <PlatformsProvider>
      <Platforms />
    </PlatformsProvider>
  );
};

export default PlatformsPage;
