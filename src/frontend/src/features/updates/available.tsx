import { ArrowRight, ExternalLink, RefreshCw } from 'lucide-react';
import type { UpdateStatusView } from '@/api/generated/api.types';
import { Timestamp } from '@/components/custom/timestamp';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useCheckInstanceUpdates } from './queries';
import { DatabaseStatusBadge, CoreStatusBadge } from './status-badge';

export function AvailableUpdateHeading({ status }: { status?: UpdateStatusView }) {
  const { mutate, isPending, isError } = useCheckInstanceUpdates();
  const formatDate = useProfileDateTimeFormatter();
  return (
    <span className="flex flex-wrap items-center gap-x-5 gap-y-2">
      <span>Available updates</span>
      <span className="ml-auto flex flex-wrap items-center justify-end gap-x-5 gap-y-2">
        <span className="text-xs font-normal text-muted-foreground">
          Last checked:{' '}
          {status?.lastAttempt ? (
            <Timestamp value={status.lastAttempt} formatDateTime={formatDate} className="text-inherit" />
          ) : (
            'Not checked'
          )}
        </span>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={isPending || !status?.checksAllowed}
          onClick={() => mutate({})}>
          <RefreshCw className={isPending ? 'size-4 animate-spin' : 'size-4'} />
          {isPending ? 'Checking…' : 'Check now'}
        </Button>
      </span>
      {isError && (
        <span role="alert" className="w-full text-right text-xs font-normal text-destructive">
          The check could not finish. Try again.
        </span>
      )}
    </span>
  );
}

export function CoreUpdateAvailability({
  data,
  isError,
  busy,
}: {
  data?: UpdateStatusView;
  isError: boolean;
  busy: boolean;
}) {
  const formatDate = useProfileDateTimeFormatter();
  if (!data)
    return <p role="status">{isError ? 'Could not load Core update status.' : 'Loading Core update status…'}</p>;
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-3 text-sm">
        <span className="font-mono break-all">{data.installedVersion}</span>
        {data.available ? (
          <>
            <ArrowRight className="size-4 text-muted-foreground" aria-hidden="true" />
            <span className="font-mono break-all">{data.available.version}</span>
            <CoreStatusBadge status={data} failed={isError} />
            <Button type="button" variant="outline" size="sm" asChild>
              <a href={data.available.releaseUrl} target="_blank" rel="noreferrer">
                Release notes <ExternalLink className="size-3.5" />
              </a>
            </Button>
          </>
        ) : (
          <CoreStatusBadge status={data} failed={isError} />
        )}
        {busy && <span className="text-muted-foreground">An update is in progress or requires recovery.</span>}
      </div>
      {(data.checkError || isError) && (
        <div role="alert">
          <AlertMessage type="warning" title="Release check failed" className="my-0">
            {data.checkError ?? 'Could not refresh Core status.'} Last successful release information is retained.
            {data.lastSuccess && (
              <span>
                {' '}
                Last successful check:{' '}
                <Timestamp value={data.lastSuccess} formatDateTime={formatDate} className="text-inherit" />
              </span>
            )}
          </AlertMessage>
        </div>
      )}
      {data.available && data.installationUnavailableReason && (
        <AlertMessage type="warning" className="my-0">
          {data.installationUnavailableReason}{' '}
          <a
            className="underline"
            href="https://docs.citadelplane.com/operations/upgrade-and-rollback/"
            target="_blank"
            rel="noreferrer">
            Manual upgrade guide
          </a>
        </AlertMessage>
      )}
    </div>
  );
}

export function DatabaseUpdateAvailability({ data }: { data?: UpdateStatusView }) {
  return (
    <div className="space-y-2 text-sm">
      <div className="flex flex-wrap items-center gap-3">
        <span>PostgreSQL {data?.databaseVersion ?? '—'}</span>
        {data?.databaseAvailable && (
          <>
            <ArrowRight className="size-4 text-muted-foreground" aria-hidden="true" />
            <span>{data.databaseAvailable}</span>
          </>
        )}
        <DatabaseStatusBadge status={data} />
      </div>
      <p className="text-muted-foreground">
        {data?.databaseManaged
          ? 'PostgreSQL 18 minor updates require review and restart Core. The existing data directory is preserved.'
          : 'In-app database updates require a registered, supported installation. This also applies when PostgreSQL runs alongside Core in Compose.'}{' '}
        <a
          className="underline"
          href="https://docs.citadelplane.com/operations/upgrade-and-rollback/#postgresql-engine-maintenance"
          target="_blank"
          rel="noreferrer">
          Database upgrade guide
        </a>
      </p>
    </div>
  );
}
