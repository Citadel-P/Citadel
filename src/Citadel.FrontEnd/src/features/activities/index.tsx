import { Activity, MoveUpRight, SquareStack } from 'lucide-react';
import { useMemo } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useActivitiesGroup } from './hooks/useActivitiesGroup';
import { ActivitiesTable } from './table';
import { ActivityEventType, ActivityResourceType, LookupResourceType } from '@/api/generated/api.types';
import { useActivityQuery } from '@/lib/atoms';
import { FilterBar, filterFieldClassName, ResourceSelectorField, SelectField } from '@/components/custom/common';
import { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { Link } from 'react-router';
import { CitadelIcons } from '@/lib/icons';

const activityResourceIcons = {
  [ActivityResourceType.Deployment]: CitadelIcons.Deployment,
  [ActivityResourceType.Registry]: CitadelIcons.Registry,
  [ActivityResourceType.Platform]: CitadelIcons.Platform,
  [ActivityResourceType.Stack]: CitadelIcons.Stack,
  [ActivityResourceType.AlertRule]: CitadelIcons.Alert,
  [ActivityResourceType.GitRepository]: CitadelIcons.GitRepository,
  [ActivityResourceType.OidcProvider]: CitadelIcons.OidcProvider,
  [ActivityResourceType.AutomationAction]: CitadelIcons.AutomationAction,
  [ActivityResourceType.User]: CitadelIcons.User,
  [ActivityResourceType.License]: CitadelIcons.License,
  [ActivityResourceType.Volume]: CitadelIcons.Volume,
  [ActivityResourceType.Build]: CitadelIcons.Build,
  [ActivityResourceType.BuildAgentPool]: CitadelIcons.BuildAgentPool,
  [ActivityResourceType.BackupPolicy]: CitadelIcons.BackupPolicy,
} satisfies Record<ActivityResourceType, any>;

const activityEventPrefixes = {
  [ActivityResourceType.Deployment]: 'Deployment',
  [ActivityResourceType.Registry]: 'Registry',
  [ActivityResourceType.Platform]: 'Platform',
  [ActivityResourceType.Stack]: 'Stack',
  [ActivityResourceType.AlertRule]: 'AlertRule',
  [ActivityResourceType.GitRepository]: 'GitRepo',
  [ActivityResourceType.OidcProvider]: 'OidcProvider',
  [ActivityResourceType.AutomationAction]: 'Action',
  [ActivityResourceType.User]: 'User',
  [ActivityResourceType.License]: 'License',
  [ActivityResourceType.Volume]: 'Volume',
  [ActivityResourceType.Build]: 'Build',
  [ActivityResourceType.BuildAgentPool]: 'BuildAgentPool',
  [ActivityResourceType.BackupPolicy]: 'BackupPolicy',
} satisfies Record<ActivityResourceType, string>;

const activityResourceLabels: Partial<Record<ActivityResourceType, string>> = {
  [ActivityResourceType.BuildAgentPool]: 'Build Agent Pool',
  [ActivityResourceType.BackupPolicy]: 'Backup Policy',
};

const activityLookupTargets = {
  [ActivityResourceType.Deployment]: LookupResourceType.Deployment,
  [ActivityResourceType.Registry]: LookupResourceType.Registry,
  [ActivityResourceType.Platform]: LookupResourceType.Platform,
  [ActivityResourceType.Stack]: LookupResourceType.Stack,
  [ActivityResourceType.AlertRule]: LookupResourceType.Alert,
  [ActivityResourceType.GitRepository]: LookupResourceType.GitRepository,
  [ActivityResourceType.OidcProvider]: LookupResourceType.OidcProvider,
  [ActivityResourceType.AutomationAction]: LookupResourceType.AutomationAction,
  [ActivityResourceType.User]: LookupResourceType.User,
  [ActivityResourceType.License]: LookupResourceType.License,
  [ActivityResourceType.Volume]: LookupResourceType.Platform,
  [ActivityResourceType.Build]: LookupResourceType.Build,
  [ActivityResourceType.BuildAgentPool]: LookupResourceType.BuildAgentPool,
  [ActivityResourceType.BackupPolicy]: LookupResourceType.BackupPolicy,
} satisfies Record<ActivityResourceType, LookupResourceType>;

export const ActivityComponents: RequiredComponents = {
  Icon: Activity,
  Content: ({ items, isLoading }) => {
    return (
      <ActivitiesTable pagedResult={items as any} isLoading={isLoading} displayTarget={true} displayPagging={true} />
    );
  },
  header: {
    showSearch: false,
    showAdd: false,
    subtitle: 'View system activy logs across your resources.',
    Extra: SearchSection,
  },
  useData: function (): ResourceDataHookResult<any> {
    const { pagedActivities, isLoading } = useActivitiesGroup();
    return { items: pagedActivities as any, isLoading, capabilities: undefined };
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
    return Object.values(ActivityResourceType).map((t) => ({
      value: t,
      label: activityResourceLabels[t] ?? t,
      icon: activityResourceIcons[t],
    }));
  }, []);

  const eventOptions = useMemo(() => {
    const all = Object.values(ActivityEventType);
    if (query.resourceType === 'All') return all;
    const eventPrefix = activityEventPrefixes[query.resourceType];
    return all.filter(
      (e) =>
        e.startsWith(eventPrefix) &&
        (query.resourceType !== ActivityResourceType.Build || !e.startsWith('BuildAgentPool')),
    );
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
      eventType: value as ActivityEventType | 'All',
      page: 1,
    });
  };

  const handleResourceChange = (value: { id: string; name: string } | undefined) => {
    setQuery({
      resourceId: value?.id,
      page: 1,
    });
  };

  return (
    <FilterBar>
      <SelectField
        value={query.resourceType}
        options={resourceOptions}
        onChange={handleResourceTypeChange}
        placeholder="All Resources"
        allLabel="All Resources"
        allIcon={SquareStack}
        className={filterFieldClassName}
      />
      {query.resourceType != 'All' && (
        <ResourceSelectorField
          targetType={activityLookupTargets[query.resourceType]}
          onSelect={handleResourceChange as any}
          selected={query.resourceId}
          placeholder={
            query.resourceType === ActivityResourceType.Volume ? 'Select Platform' : 'Select ' + query.resourceType
          }
          className={filterFieldClassName}
        />
      )}
      <SelectField
        value={query.eventType}
        options={eventOptions}
        onChange={handleEventChange}
        placeholder="All Events"
        allLabel="All Events"
        className={filterFieldClassName}
      />
    </FilterBar>
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
