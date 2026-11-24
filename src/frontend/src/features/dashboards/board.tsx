import { PreviewProvider } from './preview';
import { lazy, Suspense, useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { AppContent } from '@/components/custom/app-content';
import { PageHeader } from '@/components/custom/page-header';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Badge } from '@/components/ui/badge';
import Loader from '@/components/ui/loader';
import { useSegmentTitle } from '@/lib/atoms';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { DashboardHomePreference } from './home-preference';
import { DashboardSharing } from './sharing';
import { audiences } from './visibility';

import { DashboardView } from './view-controls';
import { supportedConfiguration } from './catalogue';
import { DashboardMenu } from './management';
import { Button } from '@/components/ui/button';
const Editor = lazy(() => import('./editor'));

export default function DashboardBoard() {
  const { id } = useParams();
  return <SavedDashboard key={id} id={id} />;
}

export function SavedDashboard({ id }: { id: string | undefined }) {
  const [editing, setEditing] = useState(false);
  const [paused, setPaused] = useState(false);
  const [viewEpoch, setViewEpoch] = useState(0);
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
    <PreviewProvider>
      <AppContent className="flex flex-col gap-(--section-gap)">
        {!edit && (
          <PageHeader
            title={saved.summary.name}
            icon={<CitadelIcons.Dashboard className="size-5" />}
            description={<Badge variant="outline">{audiences[saved.summary.visibility]}</Badge>}
            actions={
              <>
                <DashboardMenu board={saved.summary} configuration={config} />
                <DashboardHomePreference board={saved.summary} />
                {config && saved.summary.canEdit && <Button onClick={() => setEditing(true)}>Edit dashboard</Button>}
                <DashboardSharing board={saved.summary} />
              </>
            }
          />
        )}
        {config ? (
          <div hidden={!!edit}>
            <DashboardView key={viewEpoch} config={config} editing={!!edit} paused={paused} setPaused={setPaused} />
          </div>
        ) : (
          <p>This dashboard uses an unsupported configuration.</p>
        )}
        {edit && (
          <Suspense fallback={<Loader />}>
            <Editor
              board={saved.summary}
              initial={config}
              close={(changed) => {
                setEditing(false);
                if (changed) setViewEpoch((n) => n + 1);
              }}
            />
          </Suspense>
        )}
      </AppContent>
    </PreviewProvider>
  );
}
