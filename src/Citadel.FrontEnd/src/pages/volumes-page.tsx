import Volumes from '@/features/volumes';
import { VolumesProvider } from '@/features/volumes/context-provider';

const VolumesPage = () => {
  return (
    <VolumesProvider>
      <Volumes />
    </VolumesProvider>
  );
};

export default VolumesPage;
