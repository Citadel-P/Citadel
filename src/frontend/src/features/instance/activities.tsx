import { useState } from 'react';
import { ActivityResourceType } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import type { ActivityQueryState } from '@/lib/atoms';
import { useScopedActivitiesGroup } from '@/features/activities/hooks/useActivitiesGroup';
import { ScopedActivitiesTable } from '@/features/activities/table';

export function InstanceActivities() {
  const [query, setQuery] = useState<ActivityQueryState>({
    resourceType: ActivityResourceType.Instance,
    resourceId: undefined,
    eventType: 'All',
    page: 1,
    pageSize: 25,
  });
  const result = useScopedActivitiesGroup(query);
  if (result.error)
    return (
      <div role="alert">
        Could not load instance activities.{' '}
        <Button variant="outline" onClick={() => void result.refetch()}>
          Retry
        </Button>
      </div>
    );
  return (
    <ScopedActivitiesTable
      pagedResult={result.pagedActivities!}
      isLoading={result.isLoading}
      displayPagging
      query={query}
      setQuery={(patch) =>
        setQuery((previous) => ({
          ...previous,
          ...patch,
          resourceType: ActivityResourceType.Instance,
          resourceId: undefined,
          eventType: 'All',
        }))
      }
    />
  );
}
