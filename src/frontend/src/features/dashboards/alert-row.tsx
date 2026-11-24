import { useContext } from 'react';
import { Link } from 'react-router';
import { useQueryClient, useIsMutating } from '@tanstack/react-query';
import { toast } from 'sonner';
import {
  AlertEventStatus,
  ResourceType,
  type CurrentProfileAuthorizationView,
  type AlertEventView,
} from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { Button } from '@/components/ui/button';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
import { formatActivityEvent } from '@/lib/utils';
import { ViewOnlyContext } from './tile';

export function AlertRow({ row, auth }: { row: AlertEventView; auth?: CurrentProfileAuthorizationView }) {
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
  return (
    <li className="flex flex-wrap items-center justify-between gap-2 py-1.5">
      <Link className="table-link" to={'/alerts/' + row.id}>
        {row.message || formatActivityEvent(row.type)}
      </Link>
      <div className="flex items-center gap-2">
        <StateBadge kind="alertSeverity" value={row.severity} />
        {row.status === AlertEventStatus.Acknowledged && <StateBadge kind="alertEvent" value={row.status} />}
        {row.resourcePath?.startsWith('/') &&
          !row.resourcePath.startsWith('//') &&
          !row.resourcePath.includes('\\') && (
            <Link className="table-link" to={row.resourcePath}>
              Resource
            </Link>
          )}
        {can && row.status === AlertEventStatus.Active && (
          <Button
            size="sm"
            variant="outline"
            disabled={paused || !!pending}
            onClick={() => {
              if (!paused && !client.isMutating(key)) ack.mutate({ data: { ids: [row.id] } });
            }}>
            Acknowledge
          </Button>
        )}
      </div>
    </li>
  );
}
