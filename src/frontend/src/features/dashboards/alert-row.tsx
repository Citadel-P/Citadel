import { useContext } from 'react';
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
