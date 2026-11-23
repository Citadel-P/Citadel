import { Button } from '@/components/ui/button';
import { PageError } from './page-error';
import { isMaintenanceError, requestErrorMessage, requestErrorStatus } from '@/lib/request-error';

export interface ResourceReadState {
  error?: unknown;
  refetch?: () => unknown;
  isFetching?: boolean;
}

export function ResourceReadError({
  error,
  refetch,
  isFetching,
  stale = false,
  page = false,
}: ResourceReadState & { stale?: boolean; page?: boolean }) {
  const retry = refetch && (
    <Button
      variant="outline"
      size="sm"
      disabled={isFetching}
      onClick={() => {
        void refetch();
      }}>
      {isFetching ? 'Retrying…' : 'Retry'}
    </Button>
  );
  const status = requestErrorStatus(error);
  if (page && (status === 403 || status === 404)) return <PageError status={status}>{retry}</PageError>;
  if (page && !stale) {
    if (status === 500) return <PageError status={500}>{retry}</PageError>;
    if (isMaintenanceError(error))
      return (
        <PageError status={503} message={requestErrorMessage(error)}>
          {retry}
        </PageError>
      );
  }
  return (
    <div
      role="alert"
      className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm">
      <div className="space-y-1">
        <p className="font-medium">{stale ? 'Unable to refresh this resource' : 'Unable to load this resource'}</p>
        <p className="text-muted-foreground">{requestErrorMessage(error)}</p>
        {stale && <p className="text-muted-foreground">Showing previously loaded data. It may be out of date.</p>}
      </div>
      {retry}
    </div>
  );
}
