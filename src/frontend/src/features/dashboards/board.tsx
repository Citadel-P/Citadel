import { lazy, Suspense, useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { AppContent } from '@/components/custom/app-content';
import { DashboardHeader } from './header';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Badge } from '@/components/ui/badge';
import Loader from '@/components/ui/loader';
import { useSegmentTitle } from '@/lib/atoms';
import { useRead } from '@/lib/hooks';
import { audiences } from './visibility';

import { DashboardView } from './view-controls';
import { supportedConfiguration } from './catalogue';
import { DashboardMenu } from './management';
import { UnsupportedConfiguration } from './unsupported-configuration';
const Editor = lazy(() => import('./editor'));

export default function DashboardBoard() {
  const { id } = useParams();
  return <SavedDashboard key={id} id={id} />;
}

export function SavedDashboard({ id, home = false }: { id: string | undefined; home?: boolean }) {
  const [editing, setEditing] = useState(false);
  const [paused, setPaused] = useState(false);
  const board = useRead('getDashboard', { id }, { staleTime: 0 });
  const saved = board.isFetchedAfterMount ? board.data?.data : undefined;
  const name = board.isError ? undefined : saved?.summary.name;
  const [, setSegmentTitle] = useSegmentTitle();
  useEffect(() => {
    setSegmentTitle(name ? { name, action: 'Dashboard' } : null);
    return () => setSegmentTitle(null);
  }, [name, setSegmentTitle]);
  if (board.isError)
    return <ResourceReadError page error={board.error} refetch={board.refetch} isFetching={board.isFetching} />;
  if (!saved) return <Loader />;
  const config = supportedConfiguration(saved.configuration) ? saved.configuration : undefined;
  const edit = editing && config && saved.summary.canEdit;
  return (
    <AppContent className="flex flex-col gap-(--section-gap)">
      {!edit && (
        <DashboardHeader
          name={saved.summary.name}
          description={
            <>
              <Badge variant="outline">{audiences[saved.summary.visibility]}</Badge>
              {home && <span className="ml-2">Home dashboard</span>}
            </>
          }
          edit={config && saved.summary.canEdit ? () => setEditing(true) : undefined}>
          <DashboardMenu board={saved.summary} configuration={config} />
        </DashboardHeader>
      )}
      {config ? (
        <div hidden={!!edit}>
          <DashboardView boardId={id} config={config} editing={!!edit} paused={paused} setPaused={setPaused} />
        </div>
      ) : (
        <UnsupportedConfiguration />
      )}
      {edit && (
        <Suspense fallback={<Loader />}>
          <Editor board={saved.summary} initial={config} close={() => setEditing(false)} />
        </Suspense>
      )}
    </AppContent>
  );
}
