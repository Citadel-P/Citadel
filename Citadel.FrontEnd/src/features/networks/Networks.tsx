import { Network } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { AppContext } from '@/AppProvider';
import { SearchField } from '@/components/ui/SearchField';
import { AlertMessage } from '@/components/ui/alert-message';
import { PlatformStatus } from '@/api/_generated';
import { NetworksContext } from './NetworksProvider';
import NetworksTable from './NetworksTable';
import { ActionBar } from './ActionBar';

const Networks = () => {
  const platformStatus = useContextSelector(AppContext, (v) => v?.currentPlatform?.status);
  const onSearch = useContextSelector(NetworksContext, (v) => v?.onSearch)!;

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Network className="h-4 w-4" />
                <span className="sr-only">Networks</span>
              </div>
              <div className="text-md font-bold text-foreground">Networks</div>
            </div>
            <SearchField onSearch={onSearch} />
          </div>

          {/* Warning Message */}
          {platformStatus === PlatformStatus.Offline && (
            <AlertMessage type="warning" hasTitle={true}>
              This platform is not connected. Please try to update or reconnect the platform.
            </AlertMessage>
          )}
          <NetworksTable />
        </div>
      </div>
      <ActionBar />
    </div>
  );
};

export default Networks;
