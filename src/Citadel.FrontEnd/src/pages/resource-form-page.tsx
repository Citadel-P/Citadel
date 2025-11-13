import { ResourceComponents } from '@/features';
import { useRead, useResourceParamType } from '@/lib/hooks';
import { useParams } from 'react-router';
import NotFound from './not-found';
import { capitalize } from '@/lib/utils';
import { Pencil, Plus } from 'lucide-react';

export const ResourceFormPage = ({ mode }: { mode: 'create' | 'edit' }) => {
  const type = useResourceParamType()!;
  const id = useParams().id;
  const { data } = useRead('getRegistryWithConfig', { id });
  const Components = ResourceComponents[type];

  if (!Components || Components.Form === undefined) return <NotFound />;

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4 flex flex-col gap-3">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="flex items-center gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                {mode == 'create' ? <Plus className="h4 w-4" /> : <Pencil className="h4 w-4" />}
              </div>
              <div className="text-md font-bold text-foreground items-center">
                {capitalize(mode)} {capitalize(type)}
              </div>
            </div>
          </div>

          <Components.Form mode={mode} resource={data?.data} />
        </div>
      </div>
    </div>
  );
};
