import Loader from '@/components/ui/loader';
import { AddPlatformDropdown } from './AddPlatformDropdown';
import Platform from './Platform';
import { useContextSelector } from 'use-context-selector';
import { useNavigate } from 'react-router';
import { AlertMessage } from '@/components/ui/alert-message';
import { PlatformsContext } from './PlatformsProvider';
import { DeletePlatformDialog } from './dialogs/DeletePlatformDialog';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';

const Platforms = () => {
  const platforms = useContextSelector(PlatformsContext, (v) => v?.platforms);
  const isLoading = useContextSelector(PlatformsContext, (v) => v?.isLoading);
  const navigate = useNavigate();
  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h5 className="text-md font-bold text-foreground">Platforms</h5>
          </div>
          <Button
            type="button"
            onClick={() => navigate('./platforms/add')}
            className="inline-flex items-center bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> Add Platform
          </Button>
        </div>
        {isLoading && <Loader />}
        {(!platforms || !platforms.length) && !isLoading && (
          <AlertMessage type="info">
            <span>No platform has been configured yet, please add a new Docker platform</span>
            <button
              className="font-semibold underline hover:no-underline ml-1"
              onClick={() => navigate('/add-docker-platform')}>
              here
            </button>
            .
          </AlertMessage>
        )}
        {(platforms ?? []).map((platform) => (
          <Platform key={`${platform.id}`} platform={platform} />
        ))}
      </div>
      <DeletePlatformDialog />
    </div>
  );
};

export default Platforms;
