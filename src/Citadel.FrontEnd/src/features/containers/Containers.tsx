import { Box } from 'lucide-react';
import { SearchField } from '../../components/ui/SearchField';
import { ContainersTable } from './ContainersTable';
import { ContainerActionBar } from './ContainerActionBar';
import { AlertMessage } from '@/components/ui/alert-message';
import { DeleteContainerDialog } from './dialogs/DeleteContainerDialog';
import { useMemo } from 'react';
import { ContainerStateStatus } from '@/api/_generated';
import { useContainersContext } from './ContainersContext';

const Containers = () => {
  const {
    containers,
    onSearch,
    deleteIsPending: isPending,
    requestDelete,
    dialogData,
    setDialogData,
  } = useContainersContext();
  const isPlatformOffline = useMemo(
    () => containers?.some((container) => container.state === ContainerStateStatus.Offline),
    [containers],
  );

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="sm:flex sm:justify-between mb-2">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Box className="h-4 w-4" />
                <span className="sr-only">Containers</span>
              </div>
              <div className="text-md font-bold text-foreground">Containers</div>
            </div>
            <SearchField onSearch={onSearch} />
          </div>
          {isPlatformOffline && (
            <AlertMessage type="warning" hasTitle={true}>
              This platform is not connected, please try to update or reconnect the platform.{' '}
            </AlertMessage>
          )}
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainersTable />
          </div>
        </div>
      </div>
      <ContainerActionBar />
      <DeleteContainerDialog
        requestDelete={requestDelete}
        isPending={isPending}
        dialogData={dialogData}
        setDialogData={setDialogData}
      />
    </div>
  );
};

export default Containers;
