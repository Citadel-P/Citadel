import { PreviewProvider } from './preview';
import { lazy, Suspense, useState } from 'react';
import { Link } from 'react-router';
import { AppContent } from '@/components/custom/app-content';
import { PageHeader } from '@/components/custom/page-header';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { SavedDashboard } from './board';
import { CreateDashboardDialog } from './create';
import { DashboardView } from './view-controls';
import { fromTemplate } from './templates';

export default function DashboardHome() {
  const home = useRead('getDashboardDefault', undefined, { staleTime: 0, refetchOnMount: 'always' });
  if (home.isError)
    return <ResourceReadError page error={home.error} refetch={home.refetch} isFetching={home.isFetching} />;
  if (!home.isFetchedAfterMount || !home.data) return <Loader />;
  const id = home.data.data.dashboardId;
  return id ? <SavedDashboard key={id} id={id} /> : <BuiltInOverview />;
}

const Editor = lazy(() => import('./editor'));
function BuiltInOverview() {
  const [editing, setEditing] = useState(false);
  const [configuration] = useState(() => fromTemplate('Overview'));
  const [paused, setPaused] = useState(false);
  const [creating, setCreating] = useState(false);
  return (
    <PreviewProvider>
      <AppContent className="flex flex-col gap-(--section-gap)">
        {!editing && (
          <PageHeader
            title="Overview"
            icon={<CitadelIcons.Dashboard className="size-5" />}
            actions={
              <>
                <Button variant="outline" asChild>
                  <Link to="/dashboards">Dashboards</Link>
                </Button>
                <Button variant="outline" onClick={() => setEditing(true)}>
                  Edit dashboard
                </Button>
                <Button onClick={() => setCreating(true)}>Save a copy</Button>
              </>
            }
          />
        )}
        <div hidden={editing}>
          <DashboardView editing={editing} config={configuration} paused={paused} setPaused={setPaused} />
        </div>
        {editing && (
          <Suspense fallback={<Loader />}>
            <Editor initial={configuration} close={() => setEditing(false)} />
          </Suspense>
        )}
        {creating && <CreateDashboardDialog initialTemplate="Overview" close={() => setCreating(false)} />}
      </AppContent>
    </PreviewProvider>
  );
}
