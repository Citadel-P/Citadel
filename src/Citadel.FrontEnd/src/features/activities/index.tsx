import { Activity } from 'lucide-react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useActivitiesGroup } from './hooks/useActivitiesGroup';
import { ActivitiesTable } from './table';

export const ActivityComponents: RequiredComponents = {
  Icon: <Activity className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return <ActivitiesTable pagedResult={items as any} isLoading={isLoading} />;
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
  return (
    <div className="flex flex-row gap-2">
      <div>
        {/** Todo: add shadcn select based on value in the enum ActivityResourceType, also the user can select all resource  */}
      </div>
      <div>
        {/** Todo: add shadcn select based on value in the enum ActivityEventType, this select depends on the first input select, eg uf the user select 'all resource' in the first input then all ActivityEventType will be displayed, but if the user selecte 'ActivityResourceType' then only ActivityEventType that start with Deployment will be displayed */}
      </div>
    </div>
  );
}
