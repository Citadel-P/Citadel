import { AlertMessage } from '@/components/ui/alert-message';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { Plus } from 'lucide-react';
import { useNavigate } from 'react-router';

import { RegistriesContext } from './RegistriesProvider';
import { useContextSelector } from 'use-context-selector';
import { RegistriesTable } from './RegistriesTable';
import { ActionBar } from './ActionBar';

const Registries = () => {
  const navigate = useNavigate();
  const registries = useContextSelector(RegistriesContext, (v) => v?.registries);
  const isLoading = useContextSelector(RegistriesContext, (v) => v?.isLoading);

  if (isLoading) return <Loader />;
  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h5 className="text-md font-bold text-foreground">Registries</h5>
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
          <RegistriesTable />
        </div>
      </div>
      <ActionBar />
    </div>
  );
};

export default Registries;
