import { PluralResourceMap, ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { ResourceTagFilter } from '@/features/tags/components';
import { ResourcePlatformFilter } from '@/features/platforms/platform-filter';
import { Plus } from 'lucide-react';

type ResourceHeaderProps = {
  type: ResourceType;
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
  onSearch: (query: string) => void;
  onAdd: () => void;
};

export const ResourceHeader = ({
  type,
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
  onSearch,
  onAdd,
}: ResourceHeaderProps) => {
  const Icon = icon;
  return (
    <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
      <div className="flex min-w-0 items-center gap-3">
        <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          {Icon && <Icon className="h-4 w-4" />}
          <span className="sr-only">{PluralResourceMap[type]}</span>
        </div>
        <div className="flex min-w-0 flex-col">
          <div className="text-md font-bold text-foreground">{title ?? PluralResourceMap[type]}</div>
          <p className="text-xs text-muted-foreground truncate">{subtitle}</p>
        </div>
      </div>
      <div className="flex w-full min-w-0 flex-wrap items-center justify-end gap-2 md:w-auto md:flex-1">
        {showSearch && <SearchField className="w-full sm:w-64" onSearch={onSearch} />}
        {showTagFilter && <ResourceTagFilter />}
        {showPlatformFilter && <ResourcePlatformFilter />}
        {Extra && <Extra />}
        {showAdd && (
          <Button
            type="button"
            onClick={onAdd}
            disabled={addDisabled}
            className="inline-flex items-center bg-primary hover:bg-primary/80 rounded-sm text-sm px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> {addButtonTitle ?? `Add ${type}`}
          </Button>
        )}
      </div>
    </div>
  );
};
