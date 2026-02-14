import { Activity, Cable, Layers, MoveUpRight, Rocket, Server, SquareStack } from 'lucide-react';
import { useMemo } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useActivitiesGroup } from './hooks/useActivitiesGroup';
import { ActivitiesTable } from './table';
import { ActivityEventType, ActivityResourceType } from '@/api/generated/api.types';
import { useActivityQuery } from '@/lib/atoms';
import { ResourceSelectorField, SelectField } from '@/components/custom/common';
import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { Link } from 'react-router';

export const ActivityComponents: RequiredComponents = {
  Icon: <Activity className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return (
      <ActivitiesTable pagedResult={items as any} isLoading={isLoading} displayTarget={true} displayPagging={true} />
    );
  },
  header: {
    showSearch: false,
    showAdd: false,
    Extra: SearchSection,
  },
  useData: function (): ResourceDataHookResult<any> {
    const { pagedActivities, isLoading } = useActivitiesGroup();
    return { items: pagedActivities as any, isLoading };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (v) =>
        v.name?.toLowerCase().includes(s) ||
        v.id?.toLowerCase().includes(s) ||
        v.id?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};

function SearchSection() {
  const [query, setQuery] = useActivityQuery();

  const resourceOptions = useMemo(() => {
    const icons: Record<string, any> = {
      [ActivityResourceType.Deployment]: Rocket,
      [ActivityResourceType.Registry]: Cable,
      [ActivityResourceType.Platform]: Server,
      [ActivityResourceType.Stack]: Layers,
    };

    return Object.values(ActivityResourceType).map((t) => ({
      value: t,
      label: t,
      icon: icons[t],
    }));
  }, []);

  const eventOptions = useMemo(() => {
    const all = Object.values(ActivityEventType);
    if (query.resourceType === 'All') return all;
    return all.filter((e) => e.startsWith(query.resourceType));
  }, [query.resourceType]);

  const handleResourceTypeChange = (value: string) => {
    setQuery({
      ...query,
      resourceType: value as ActivityResourceType | 'All',
      resourceId: undefined,
      eventType: 'All',
      page: 1,
    });
  };

  const handleEventChange = (value: string) => {
    setQuery({
      ...query,
      eventType: value as ActivityEventType | 'All',
      page: 1,
    });
  };

  const handleResourceChange = (v: { id: string; name: string }) => {
    setQuery({
      ...query,
      resourceId: v.id,
      page: 1,
    });
  };

  return (
    <div className="flex flex-col sm:flex-row sm:gap-4 gap-2">
      <SelectField
        value={query.resourceType}
        options={resourceOptions}
        onChange={handleResourceTypeChange}
        placeholder="All Resources"
        allLabel="All Resources"
        allIcon={SquareStack}
      />
      {query.resourceType != 'All' && (
        <ResourceSelectorField
          type={query.resourceType as any}
          onSelect={handleResourceChange as any}
          selected={query.resourceId}
          placeholder={'Select ' + query.resourceType}
          className="w-[200px]"
        />
      )}
      <SelectField
        value={query.eventType}
        options={eventOptions}
        onChange={handleEventChange}
        placeholder="All Events"
        allLabel="All Events"
      />
    </div>
  );
}

export function ActivitiesTab({ resourceId, resourceType }: { resourceId: string; resourceType: ResourceType }) {
  const { pagedActivities, isLoading } = useActivitiesGroup(resourceId, resourceType, 20);
  const link = useMemo(
    () => `../activities?resourceType=${resourceType}&resourceId=${resourceId}`,
    [resourceId, resourceType],
  );
  return (
    <div className="flex flex-col gap-4">
      <ActivitiesTable pagedResult={pagedActivities as any} isLoading={isLoading} />
      <Link
        to={link}
        className="border border-dashed p-2 rounded-md flex items-center justify-center text-muted-foreground gap-2 cursor-pointer">
        <span>Show All</span>
        <MoveUpRight className="h-3.5 w-3.5" />
      </Link>
      <TaskSheet type={'Activity'} />
    </div>
  );
}
