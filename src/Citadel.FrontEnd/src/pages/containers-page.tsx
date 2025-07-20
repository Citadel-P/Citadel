import Containers from '@/features/containers/Containers';
import ContainersProvider from '@/features/containers/ContainersProvider';

const ContainersPage = () => {
  return (
    <ContainersProvider>
      <Containers />
    </ContainersProvider>
  );
};

export default ContainersPage;
