import Volumes from '@/features/volumes/Volumes';
import VolumesProvider from '@/features/volumes/VolumesProvider';

const VolumesPage = () => {
  return (
    <VolumesProvider>
      <Volumes />
    </VolumesProvider>
  );
};

export default VolumesPage;
