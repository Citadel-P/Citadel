import { AlertMessage } from '@/components/ui/alert-message';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { Plus } from 'lucide-react';
import { useNavigate } from 'react-router';


const Registries = () => {
  const navigate = useNavigate();
  const registries = [];
  const isLoading = true;

  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h5 className="text-md font-bold text-foreground">Registries</h5>
          </div>
          <Button
            type="button"
            onClick={() => navigate('/add-registry')}
            className="inline-flex items-center dark:text-foreground bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> Add registy
          </Button>
        </div>
        {isLoading && <Loader />}
        {registries?.length === 0 && !isLoading && (
          <AlertMessage type="info">
            <span>No registry has been configured yet, please add a new registry</span>
            <button
              className="font-semibold underline hover:no-underline ml-1"
              onClick={() => navigate('/add-registry')}>
              here
            </button>
            .
          </AlertMessage>
        )}

      
      </div>
    </div>
  );
};

export default Registries;
