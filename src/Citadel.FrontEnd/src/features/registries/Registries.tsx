import { AlertMessage } from '@/components/ui/alert-message';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { Cable, Plus } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useRegistriesContext } from './RegistriesContext';
import { RegistriesTable } from './RegistriesTable';
import { ActionBar } from './action-bar';

export default function Registries() {
  const navigate = useNavigate();
  const { registries, isLoading } = useRegistriesContext();

  if (isLoading) return <Loader />;
  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="mb-4 flex items-center justify-between">
            <div className="flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Cable className="h-4 w-4" />
                <span className="sr-only">Registries</span>
              </div>
              <div className="text-md font-bold text-foreground">Registries</div>
            </div>
            <Button
              type="button"
              onClick={() => navigate('/registries/add')}
              className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
              <Plus className="h-3 w-3" /> Add Registy
            </Button>
          </div>
          {!isLoading && registries?.length === 0 && (
            <AlertMessage type="info">
              <span>No registry has been configured yet, please add a new registry</span>
              <button
                className="font-semibold underline hover:no-underline ml-1"
                onClick={() => navigate('/registries/add')}>
                here
              </button>
              .
            </AlertMessage>
          )}
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <RegistriesTable />
          </div>
        </div>
      </div>
      <ActionBar />
    </div>
  );
}
