import Images from '@/features/images/Images';
import { ImagesProvider } from '@/features/images/ImagesProvider';

const ImagesPage = () => {
  return (
    <ImagesProvider>
      <Images />
    </ImagesProvider>
  );
};

export default ImagesPage;
