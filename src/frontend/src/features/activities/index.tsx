import { Activity, MoveUpRight, SquareStack } from 'lucide-react';
import { useMemo } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useActivitiesGroup } from './hooks/useActivitiesGroup';
import { ActivitiesTable } from './table';
import { ActivityEventType, ActivityResourceType, LookupResourceType } from '@/api/generated/api.types';
import { ActivityFiltersState, useActivityQuery } from '@/lib/atoms';
import { FilterBar, filterFieldClassName, ResourceSelectorField, SelectField } from '@/components/custom/common';
import TaskSheet from '@/components/custom/task-sheet';
import { Link } from 'react-router';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';

const activityResourceIcons = {
  [ActivityResourceType.Deployment]: CitadelIcons.Deployment,
  [ActivityResourceType.Registry]: CitadelIcons.Registry,
  [ActivityResourceType.Platform]: CitadelIcons.Platform,
  [ActivityResourceType.Stack]: CitadelIcons.Stack,
  [ActivityResourceType.AlertRule]: CitadelIcons.Alert,
  [ActivityResourceType.GitRepository]: CitadelIcons.GitRepository,
  [ActivityResourceType.OidcProvider]: CitadelIcons.OidcProvider,
  [ActivityResourceType.Script]: CitadelIcons.Script,
  [ActivityResourceType.User]: CitadelIcons.User,
  [ActivityResourceType.Team]: CitadelIcons.Team,
  [ActivityResourceType.Role]: CitadelIcons.Role,
  [ActivityResourceType.License]: CitadelIcons.License,
  [ActivityResourceType.Volume]: CitadelIcons.Volume,
  [ActivityResourceType.Build]: CitadelIcons.Build,
  [ActivityResourceType.BuildAgentPool]: CitadelIcons.BuildAgentPool,
  [ActivityResourceType.BackupPolicy]: CitadelIcons.BackupPolicy,
  [ActivityResourceType.SwarmService]: CitadelIcons.SwarmService,
  [ActivityResourceType.ServiceAccount]: CitadelIcons.ServiceAccount,
} satisfies Record<ActivityResourceType, any>;

const activityEventPrefixes = {
  [ActivityResourceType.Deployment]: 'Deployment',
  [ActivityResourceType.Registry]: 'Registry',
  [ActivityResourceType.Platform]: 'Platform',
  [ActivityResourceType.Stack]: 'Stack',
  [ActivityResourceType.AlertRule]: 'AlertRule',
  [ActivityResourceType.GitRepository]: 'GitRepo',
  [ActivityResourceType.OidcProvider]: 'OidcProvider',
  [ActivityResourceType.Script]: 'Script',
  [ActivityResourceType.User]: 'User',
  [ActivityResourceType.Team]: 'Team',
  [ActivityResourceType.Role]: 'Role',
  [ActivityResourceType.License]: 'License',
  [ActivityResourceType.Volume]: 'Volume',
  [ActivityResourceType.Build]: 'Build',
  [ActivityResourceType.BuildAgentPool]: 'BuildAgentPool',
  [ActivityResourceType.BackupPolicy]: 'Backup',
  [ActivityResourceType.SwarmService]: 'SwarmService',
  [ActivityResourceType.ServiceAccount]: 'ServiceAccount',
} satisfies Record<ActivityResourceType, string>;

const activityResourceLabels: Partial<Record<ActivityResourceType, string>> = {
  [ActivityResourceType.BuildAgentPool]: 'Build Agent Pool',
  [ActivityResourceType.BackupPolicy]: 'Backup Policy',
  [ActivityResourceType.ServiceAccount]: 'Service Account',
};

const activityLookupTargets = {
  [ActivityResourceType.Deployment]: LookupResourceType.Deployment,
  [ActivityResourceType.Registry]: LookupResourceType.Registry,
  [ActivityResourceType.Platform]: LookupResourceType.Platform,
  [ActivityResourceType.Stack]: LookupResourceType.Stack,
  [ActivityResourceType.AlertRule]: LookupResourceType.Alert,
  [ActivityResourceType.GitRepository]: LookupResourceType.GitRepository,
  [ActivityResourceType.OidcProvider]: LookupResourceType.OidcProvider,
  [ActivityResourceType.Script]: LookupResourceType.Script,
  [ActivityResourceType.User]: LookupResourceType.User,
  [ActivityResourceType.Team]: LookupResourceType.Team,
  [ActivityResourceType.Role]: LookupResourceType.Role,
  [ActivityResourceType.License]: LookupResourceType.License,
  [ActivityResourceType.Volume]: LookupResourceType.Platform,
  [ActivityResourceType.Build]: LookupResourceType.Build,
  [ActivityResourceType.BuildAgentPool]: LookupResourceType.BuildAgentPool,
  [ActivityResourceType.BackupPolicy]: LookupResourceType.BackupPolicy,
  [ActivityResourceType.SwarmService]: LookupResourceType.SwarmService,
  [ActivityResourceType.ServiceAccount]: LookupResourceType.ServiceAccount,
} satisfies Record<ActivityResourceType, LookupResourceType>;

const administratorActivityResourceTypes = new Set<ActivityResourceType>([
  ActivityResourceType.OidcProvider,
  ActivityResourceType.User,
  ActivityResourceType.Team,
  ActivityResourceType.Role,
  ActivityResourceType.License,
  ActivityResourceType.ServiceAccount,
]);

export const getActivityResourceOptions = (isAdministrator: boolean) =>
  Object.values(ActivityResourceType)
    .filter((type) => isAdministrator || !administratorActivityResourceTypes.has(type))
    .map((type) => ({
      value: type,
      label: activityResourceLabels[type] ?? type,
      icon: activityResourceIcons[type],
    }));

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
    const { pagedActivities, isLoading, error, refetch, isFetching } = useActivitiesGroup();
    return { error, refetch, isFetching, items: pagedActivities as any, isLoading, capabilities: undefined };
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

export function getActivityEventOptions(resourceType: ActivityFiltersState['resourceType']) {
  const all = Object.values(ActivityEventType);
  const eventPrefix = resourceType === 'All' ? '' : activityEventPrefixes[resourceType];
  const events =
    resourceType === 'All'
      ? all
      : all.filter(
          (e) =>
            e.startsWith(eventPrefix) &&
            (resourceType !== ActivityResourceType.Build || !e.startsWith('BuildAgentPool')),
        );
  return events.flatMap<string | { value: string; label: string }>((event) =>
    event === ActivityEventType.BackupRunCompleted
      ? [event, { value: 'BackupRunFailed', label: 'Backup run failed' }]
      : [event],
  );
}

function SearchSection() {
  const [query, setQuery] = useActivityQuery();
  const { data: profileResponse } = useRead('getCurrentProfile');
  const isAdministrator = profileResponse?.data.authorization.isAdministrator === true;

  const resourceOptions = useMemo(() => getActivityResourceOptions(isAdministrator), [isAdministrator]);

  const eventOptions = useMemo(() => getActivityEventOptions(query.resourceType), [query.resourceType]);

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
      eventType: value as ActivityFiltersState['eventType'],
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

export function ActivitiesTab({
  resourceId,
  resourceType,
}: {
  resourceId: string;
  resourceType: `${ActivityResourceType}`;
}) {
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
