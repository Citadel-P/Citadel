import { Network, Plus } from 'lucide-react';
import { SearchField } from '@/components/ui/SearchField';
import { useNetworksContext } from './NetworksContext';
import NetworksTable from './NetworksTable';
import { NetworksActionBar } from './action-bar';
import { Button } from '@/components/ui/button';
import { useNavigate } from 'react-router';

const Networks = () => {
  const navigate = useNavigate();
  const { onSearch } = useNetworksContext();

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
            <div className="flex gap-2">
              <SearchField onSearch={onSearch} />
              <Button
                type="button"
                onClick={() => navigate('./add')}
                className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
                <Plus className="h-3 w-3" /> Add Network
              </Button>
            </div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <NetworksTable />
          </div>
        </div>
      </div>
      <NetworksActionBar />
    </div>
  );
};

export default Networks;
