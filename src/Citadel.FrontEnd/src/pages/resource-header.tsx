import { PluralResourceMap, ResourceType } from '@/api/types';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';

type ResourceHeaderProps = {
  type: ResourceType;
  icon?: React.ComponentType<{ className?: string }>;
  title?: string;
  subtitle?: string;
  showSearch?: boolean;
  showAdd?: boolean;
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
  addButtonTitle,
  Extra,
  onSearch,
  onAdd,
}: ResourceHeaderProps) => {
  const Icon = icon;
  return (
    <div className="flex flex-col sm:flex-row gap-2 sm:justify-between">
      <div className="flex items-center gap-3">
        <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          {Icon && <Icon className="h-4 w-4" />}
          <span className="sr-only">{PluralResourceMap[type]}</span>
        </div>
        <div className="flex flex-col">
          <div className="text-md font-bold text-foreground">{title ?? PluralResourceMap[type]}</div>
          <p className="text-xs text-muted-foreground truncate">{subtitle}</p>
        </div>
      </div>
      <div className="flex gap-2">
        {showSearch && <SearchField onSearch={onSearch} />}
        {showAdd && (
          <Button
            type="button"
            onClick={onAdd}
            className="inline-flex items-center bg-primary hover:bg-primary/80 rounded-sm text-sm px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> {addButtonTitle ?? `Add ${type}`}
          </Button>
        )}
        {Extra && <Extra />}
      </div>
    </div>
  );
};
