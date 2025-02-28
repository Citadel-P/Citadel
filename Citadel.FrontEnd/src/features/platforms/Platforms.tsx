import Loader from '@/components/ui/loader';
import { AddPlatformDropdown } from './AddPlatformDropdown';
import Platform from './Platform';
import { PlatformsContext } from './PlatformsProvider';
import { useContextSelector } from 'use-context-selector';
import { useNavigate } from 'react-router';
import { AlertMessage } from '@/components/ui/alert-message';

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
          <AddPlatformDropdown />
        </div>
        {isLoading && <Loader />}
        {platforms?.length === 0 && !isLoading && (
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
    </div>
  );
};

export default Platforms;
