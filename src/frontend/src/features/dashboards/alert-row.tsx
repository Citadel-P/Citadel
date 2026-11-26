import { Sheet, SheetTrigger, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { PaginationControls } from '@/components/custom/common';
import { useAlertPage } from '@/features/alerters/alert-events/hooks/useAlertEventsList';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import Loader from '@/components/ui/loader';
import { type ReactNode, useContext, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { useQueryClient, useIsMutating } from '@tanstack/react-query';
import { toast } from 'sonner';
import {
  AlertEventStatus,
  ResourceType,
  type CurrentProfileAuthorizationView,
  type AlertEventView,
} from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { TimelineItem } from './timeline';
import { DropdownActions } from '@/components/custom/dropdown-with-dialog';
import { Check, ExternalLink } from 'lucide-react';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
import { formatActivityEvent } from '@/lib/utils';
import { ViewOnlyContext } from './tile';

import type { DateTimeFormatter } from '@/lib/date-time';
export function AlertRow({
  row,
  auth,
  format,
}: {
  format: DateTimeFormatter;
  row: AlertEventView;
  auth?: CurrentProfileAuthorizationView;
}) {
  const paused = useContext(ViewOnlyContext);
  const client = useQueryClient();
  const can =
    auth?.isAdministrator ||
    (auth?.writableResourceTypes?.includes(ResourceType.Alert) && row.resourceType !== 'Updates');
  const key = { mutationKey: ['acknowledgeAlertEvents', row.id] };
  const pending = useIsMutating(key);
  const ack = useMutate('acknowledgeAlertEvents', {
    ...key,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['listAlertEvents'] });
      void client.invalidateQueries({ queryKey: ['getAlertEvent', { id: row.id }] });
    },
    onError: (e) => toast.error(requestErrorMessage(e)),
  });
  const navigate = useNavigate();
  const path = '/alerts/' + row.id;
  const resource = row.resourcePath;
  const safe = resource?.startsWith('/') && !resource.startsWith('//') && !resource.includes('\\');
  return (
    <TimelineItem
      format={format}
      type={row.resourceType}
      title={row.message || formatActivityEvent(row.type)}
      path={path}
      time={row.createdAt}
      status={
        <>
          <StateBadge kind="alertSeverity" value={row.severity} />
          {row.status === AlertEventStatus.Acknowledged && <StateBadge kind="alertEvent" value={row.status} />}
        </>
      }
      context={
        safe &&
        resource && (
          <Link className="table-link truncate" to={resource}>
            {row.resourceName || 'Resource'}
          </Link>
        )
      }
      actions={
        <DropdownActions
          items={[
            { id: 'open', label: 'Alert details', icon: <ExternalLink />, onClick: () => navigate(path) },
            ...(can && row.status === AlertEventStatus.Active
              ? [
                  {
                    id: 'ack',
                    label: 'Acknowledge',
                    icon: <Check />,
                    disabled: paused || !!pending,
                    onClick: () => {
                      if (!paused && !client.isMutating(key)) ack.mutate({ data: { ids: [row.id] } });
                    },
                  },
                ]
              : []),
          ]}
        />
      }
    />
  );
}

export function ScopedAlerts({ tags, children }: { tags: string[]; children: ReactNode }) {
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button variant="ghost" className="h-auto w-fit justify-start gap-2 px-1 text-left">
          {children}
        </Button>
      </SheetTrigger>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>Open alerts</SheetTitle>
        </SheetHeader>
        <ScopedAlertList tags={tags} />
      </SheetContent>
    </Sheet>
  );
}
function ScopedAlertList({ tags }: { tags: string[] }) {
  const [page, setPage] = useState(1);
  const format = useProfileDateTimeFormatter();
  const { error, pagedAlertEvents: result } = useAlertPage(
    { page, pageSize: 25, resourceId: undefined, resourceType: 'All', alertType: 'All', unresolvedOnly: true },
    undefined,
    tags.length ? tags : undefined,
  );
  if (error) return <p>Open alerts unavailable.</p>;
  if (!result) return <Loader />;
  const pages = Math.ceil(result.totalCount / 25);
  return (
    <div className="min-h-0 overflow-auto px-4">
      {result.items.length ? (
        <ul>
          {result.items.map((row) => (
            <AlertRow key={row.id} row={row} format={format} />
          ))}
        </ul>
      ) : (
        <p>No open alerts.</p>
      )}
      {pages > 1 && <PaginationControls currentPage={page} onPageChange={setPage} totalPages={pages} />}
    </div>
  );
}
