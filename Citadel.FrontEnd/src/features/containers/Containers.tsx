import { Container } from 'lucide-react';
import { SearchField } from './SearchField';
import { ContainersTable } from './ContainersTable';
import { ActionBar } from './ActionBar';
import { useContextSelector } from 'use-context-selector';
import { AlertMessage } from '@/components/ui/alert-message';
import { DeleteContainerDialog } from './dialogs/DeleteContainerDialog';
import { PlatformStatus } from '@/api/_generated';
import { AppContext } from '@/AppProvider';

const Containers = () => {
  const platformStatus = useContextSelector(AppContext, (v) => v?.currentPlatform?.status);

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="sm:flex sm:justify-between mb-2">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Container className="h-4 w-4" />
                <span className="sr-only">Containers</span>
              </div>
              <div className="text-md font-bold text-foreground">Containers</div>
            </div>
            <SearchField />
          </div>
          {platformStatus === PlatformStatus.Offline && (
            <AlertMessage type="warning" hasTitle={true}>
              This platform is not connected, please try to update or reconnect the platform.{' '}
            </AlertMessage>
          )}
          <ContainersTable />
        </div>
      </div>
      <ActionBar />
      <DeleteContainerDialog />
    </div>
  );
};

export default Containers;
