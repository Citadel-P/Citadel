import Loader from '@/components/ui/loader';
import { useNavigate } from 'react-router';
import { AlertMessage } from '@/components/custom/alert-message';
import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { DockerPlatform } from './docker-platform';
import { ActionData } from '@/pages/types';

export const Platforms = ({
  items,
  actions,
  isLoading,
  isFiltered,
}: {
  items: PlatformView[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions: Record<
    string,
    React.FC<{ resource: PlatformView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const navigate = useNavigate();

  return (
    <>
      {isLoading && <Loader />}
      {(!items || !items.length) &&
        !isLoading &&
        (isFiltered ? (
          <div className="flex items-center justify-center p-4">
            <div className="rounded-md bg-muted/20 text-sm p-2 px-3 text-center">
              No platforms match the current filters.
            </div>
          </div>
        ) : (
          <AlertMessage type="info">
            <span>
              No platform is currently configured. Add a
              <button
                className="font-semibold underline hover:no-underline ml-1"
                onClick={() => navigate('/platforms/add')}>
                new platform
              </button>
              &nbsp; to get started.
            </span>
          </AlertMessage>
        ))}
      {(items ?? []).map(
        (platform) =>
          (platform.type === PlatformType.Docker || platform.type === PlatformType.DockerSwarm) && (
            <div key={`${platform.id}`} className="space-y-1 rounded-sm shadow-xs">
              <DockerPlatform platform={platform} actions={actions} />
            </div>
          ),
      )}
    </>
  );
};
