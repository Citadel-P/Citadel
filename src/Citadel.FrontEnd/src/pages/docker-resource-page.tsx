import { useDockerResourceParamType } from '@/lib/hooks';
import { useNavigate, useParams } from 'react-router';
import { DockerResourceComponents, RequiredDockerComponents } from './types';
import { DockerResourceType, PluralResourceMap } from '@/api/types';
import { SearchField } from '@/components/ui/SearchField';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import NotFound from './NotFound';
import { useMemo, useState } from 'react';

const DockerResourcePage = () => {
  const platformId = useParams().platformId ?? '';
  const type = useDockerResourceParamType()!;

  const Components = DockerResourceComponents[type];
  if (!Components) return <NotFound />;

  return <ResourceView key={type} Components={Components} platformId={platformId} type={type} />;
};

const ResourceView = <T,>({ Components, platformId, type }: ResourceViewProps<T>) => {
  const navigate = useNavigate();

  const { items, isLoading } = Components.useData(platformId);
  const [search, setSearch] = useState('');
  const filtered = useMemo(() => {
    return Components.filterItems ? Components.filterItems(items, search) : items;
  }, [items, search, Components]);

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                {Components.Icon}
                <span className="sr-only">{PluralResourceMap[type]}</span>
              </div>
              <div className="text-md font-bold text-foreground">{PluralResourceMap[type]}</div>
            </div>
            <div className="flex gap-2">
              <SearchField onSearch={setSearch} />
              <Button
                type="button"
                onClick={() => navigate('./add')}
                className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
                <Plus className="h-3 w-3" /> Add {type}
              </Button>
            </div>
          </div>

          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <Components.Table items={filtered ?? []} isLoading={isLoading} />
          </div>
        </div>
      </div>

      <Components.ActionBar items={items} />
      <Components.DeleteDialog />
    </div>
  );
};

type ResourceViewProps<T = any> = {
  Components: RequiredDockerComponents<T>;
  platformId: string;
  type: DockerResourceType;
};

export default DockerResourcePage;
