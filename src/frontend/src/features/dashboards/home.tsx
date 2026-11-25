import { lazy, Suspense, useState } from 'react';
import { Link } from 'react-router';
import { AppContent } from '@/components/custom/app-content';
import { DashboardHeader } from './header';
import { MoreHorizontal } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from '@/components/ui/dropdown-menu';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
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
  return id ? <SavedDashboard key={id} id={id} home /> : <BuiltInOverview />;
}

const Editor = lazy(() => import('./editor'));
function BuiltInOverview() {
  const [editing, setEditing] = useState(false);
  const [configuration] = useState(() => fromTemplate('Overview'));
  const [paused, setPaused] = useState(false);
  const [creating, setCreating] = useState(false);
  return (
    <AppContent className="flex flex-col gap-(--section-gap)">
      {!editing && (
        <DashboardHeader name="Overview" description="Built-in dashboard" edit={() => setEditing(true)}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon" aria-label="Dashboard options">
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => setCreating(true)}>Save a copy</DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to="/dashboards">Dashboards</Link>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </DashboardHeader>
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
  );
}
