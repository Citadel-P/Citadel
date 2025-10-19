import Networks from '@/features/networks';
import { NetworksProvider } from '@/features/networks/context-provider';

const NetworksPage = () => {
  return (
    <NetworksProvider>
      <Networks />
    </NetworksProvider>
  );
};

export default NetworksPage;
