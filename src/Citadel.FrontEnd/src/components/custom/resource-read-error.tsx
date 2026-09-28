import { Button } from '@/components/ui/button';
import { requestErrorMessage } from '@/lib/request-error';

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
}: ResourceReadState & { stale?: boolean }) {
  return (
    <div
      role="alert"
      className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm">
      <div className="space-y-1">
        <p className="font-medium">{stale ? 'Unable to refresh this resource' : 'Unable to load this resource'}</p>
        <p className="text-muted-foreground">{requestErrorMessage(error)}</p>
        {stale && <p className="text-muted-foreground">Showing previously loaded data. It may be out of date.</p>}
      </div>
      {refetch && (
        <Button
          variant="outline"
          size="sm"
          disabled={isFetching}
          onClick={() => {
            void refetch();
          }}>
          {isFetching ? 'Retrying…' : 'Retry'}
        </Button>
      )}
    </div>
  );
}
