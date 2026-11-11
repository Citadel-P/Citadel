import { PageHeader } from '@/components/custom/page-header';
import { PluralResourceMap, ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { ResourceTagFilter } from '@/features/tags/components';
import { ResourcePlatformFilter } from '@/features/platforms/platform-filter';
import { cn } from '@/lib/utils';
import { Plus } from 'lucide-react';

type ResourceHeaderProps = {
  type: string;
  variant?: 'plain' | 'card';
  Details?: React.FC;
  icon?: React.ComponentType<{ className?: string }>;
  title?: string;
  subtitle?: string;
  showSearch?: boolean;
  showAdd?: boolean;
  showTagFilter?: boolean;
  showPlatformFilter?: boolean;
  addDisabled?: boolean;
  addButtonTitle?: string;
  Extra?: React.FC;
  Actions?: React.FC;
  onSearch: (query: string) => void;
  onAdd: () => void;
};

export const ResourceHeader = ({
  type,
  variant,
  Details,
  icon,
  title,
  subtitle,
  showSearch,
  showAdd,
  showTagFilter,
  showPlatformFilter,
  addDisabled,
  addButtonTitle,
  Extra,
  Actions,
  onSearch,
  onAdd,
}: ResourceHeaderProps) => {
  const Icon = icon;
  const defaultTitle = PluralResourceMap[type as ResourceType] ?? type;
  return (
    <div className="space-y-(--section-gap)">
      <div className={cn(variant === 'card' && 'rounded-lg border bg-card p-(--surface-padding) shadow-xs')}>
        <PageHeader
          title={title ?? defaultTitle}
          description={subtitle}
          icon={Icon && <Icon className="size-5" />}
          actions={
            (showAdd || Actions) && (
              <>
                {showAdd && (
                  <Button type="button" onClick={onAdd} disabled={addDisabled}>
                    <Plus className="h-3 w-3" /> {addButtonTitle ?? `Add ${type}`}
                  </Button>
                )}
                {Actions && <Actions />}
              </>
            )
          }
        />
        {Details && (
          <div className="mt-3 sm:pl-14">
            <Details />
          </div>
        )}
      </div>
      {(showSearch || showTagFilter || showPlatformFilter || Extra) && (
        <div
          role="region"
          aria-label={`${title ?? defaultTitle} filters`}
          className="flex min-w-0 flex-wrap items-center gap-2 rounded-lg border bg-card p-3 shadow-xs">
          {showSearch && (
            <SearchField
              placeholder={`Search ${defaultTitle.toLowerCase()}…`}
              className="mb-0 w-full sm:w-72"
              onSearch={onSearch}
            />
          )}
          <div className="flex min-w-0 flex-wrap items-center gap-2 sm:ml-auto">
            {showTagFilter && <ResourceTagFilter />}
            {showPlatformFilter && <ResourcePlatformFilter />}
            {Extra && <Extra />}
          </div>
        </div>
      )}
    </div>
  );
};
