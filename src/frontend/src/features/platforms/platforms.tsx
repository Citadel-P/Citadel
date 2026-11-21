import { useRealtimeContext } from '@/lib/context/realtime-context';
import { Button } from '@/components/ui/button';
import { useState } from 'react';
import { ResourceCardsSkeleton } from '@/pages/resource-skeleton';
import { useNavigate } from 'react-router';
import { AlertMessage } from '@/components/custom/alert-message';
import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { DockerPlatform } from './docker-platform';
import { ActionData } from '@/pages/types';

export const Platforms = ({
  items,
  actions,
  isLoading,
  isFiltered,
}: {
  items: PlatformView[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions: Record<
    string,
    React.FC<{ resource: PlatformView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const navigate = useNavigate();
  const { groups } = useRealtimeContext();
  const [retrying, setRetrying] = useState(false);
  const retry = async () => {
    if (!groups) return;
    setRetrying(true);
    try {
      await groups.retryGroup('platforms');
    } finally {
      setRetrying(false);
    }
  };

  return (
    <>
      {groups?.capacityGroups.has('platforms') && (
        <AlertMessage
          type="warning"
          action={
            <Button variant="outline" size="sm" disabled={retrying} onClick={() => void retry().catch(console.warn)}>
              {retrying ? 'Retrying…' : 'Retry live updates'}
            </Button>
          }>
          Live updates paused because the platform list exceeds the snapshot limit. Displayed platforms may be out of
          date.
        </AlertMessage>
      )}
      {isLoading && !items?.length && <ResourceCardsSkeleton />}
      {(!items || !items.length) &&
        !isLoading &&
        (isFiltered ? (
          <div className="flex items-center justify-center p-4">
            <div className="rounded-md bg-muted/20 text-sm p-2 px-3 text-center">
              No platforms match the current filters.
            </div>
          </div>
        ) : (
          <AlertMessage type="info">
            <span>
              No platform is currently configured. Add a
              <button
                className="font-semibold underline hover:no-underline ml-1"
                onClick={() => navigate('/platforms/add')}>
                new platform
              </button>
              &nbsp; to get started.
            </span>
          </AlertMessage>
        ))}
      {(items ?? []).map(
        (platform) =>
          (platform.type === PlatformType.Docker || platform.type === PlatformType.DockerSwarm) && (
            <div key={`${platform.id}`} className="space-y-1 rounded-sm shadow-2xs">
              <DockerPlatform platform={platform} actions={actions} />
            </div>
          ),
      )}
    </>
  );
};
