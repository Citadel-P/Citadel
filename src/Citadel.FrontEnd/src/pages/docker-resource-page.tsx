import { useDockerResourceParamType, useRead } from '@/lib/hooks';
import { useNavigate, useParams } from 'react-router';
import { DockerResourceComponents } from './types';
import { PluralResourceMap } from '@/api/types';
import { SearchField } from '@/components/ui/SearchField';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import NotFound from './NotFound';
import { useCallback, useEffect, useState } from 'react';

const DockerResourcePage = () => {
  const navigate = useNavigate();
  const platformId = useParams().platformId;

  const type = useDockerResourceParamType()!;
  const { data, isLoading } = useRead(`list${type}s`, { platformId: platformId });
  const [items, setItems] = useState<any[]>([]);
  const [originalItems, setOriginalItems] = useState<any[]>([]);
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  useEffect(() => {
    if (data?.data) {
      const result = data?.data[PluralResourceMap[type].toLowerCase() as keyof typeof data.data] ?? [];
      setItems(result);
      setOriginalItems(result);
    }
  }, [data, type]);

  useEffect(() => {
    if (!originalItems?.length) return;

    if (currentSearchTerm.trim() === '') {
      setItems(originalItems);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();
      // filter by name or id containing the search term
      const filtered = originalItems.filter((item) => {
        const nameMatches = item.name?.toLowerCase().includes(searchLower) || false;
        const idMatches =
          (item.id && item.id.substring(0, 12).toLowerCase().includes(searchLower)) ||
          (item.id && item.id.toLowerCase().includes(searchLower)) ||
          false;

        return nameMatches || idMatches;
      });

      setItems(filtered);
    }
  }, [originalItems, currentSearchTerm]);

  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  const Components = DockerResourceComponents[type];
  if (!Components) {
    return <NotFound />;
  }

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
              <SearchField onSearch={onSearch} />
              <Button
                type="button"
                onClick={() => navigate('./add')}
                className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
                <Plus className="h-3 w-3" /> Add {type}
              </Button>
            </div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <Components.Table items={items} isLoading={isLoading} />
          </div>
        </div>
      </div>
      <Components.ActionBar items={items} />
      <Components.DeleteDialog />
    </div>
  );
};

export default DockerResourcePage;
