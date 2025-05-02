import Networks from '@/features/networks/Networks';
import NetworksProvider from '@/features/networks/NetworksProvider';

const NetworksPage = () => {
  return (
    <NetworksProvider>
      <Networks />
    </NetworksProvider>
  );
};

export default NetworksPage;
