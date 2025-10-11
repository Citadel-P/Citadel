import { PlatformDescriptorDockerPlatformDescriptor, PlatformStatus, PlatformView } from '@/api/generated/api.types';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Power, PowerOff, CirclePause, Pencil, Trash2 } from 'lucide-react';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';
import { usePlatformsContext } from './PlatformsContext';

const DockerPlatform = ({ platform }: { platform: PlatformView }) => {
  const { setDialogData } = usePlatformsContext();
  const isPlatfomOnline = platform.status === PlatformStatus.Online;
  const LastSnapshotTooltip = () => {
    const lastSnapshot = platform.stats?.at(0)?.created
      ? new Date(platform.stats[0].created! * 1000).getTime()
      : new Date().getTime();
    return (
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div
              className={`absolute right-2 top-1.5 ${isPlatfomOnline ? 'bg-green-500' : 'bg-red-500'} h-3.5 w-3.5 rounded-full border-2`}></div>
          </TooltipTrigger>
          <TooltipContent>
            <p>{fromNow(lastSnapshot)} (Last snapshot)</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  };

  const ContainersPaused = () => {
    return (
      <div className="flex items-center">
        <TooltipProvider delayDuration={200}>
          <Tooltip>
            <TooltipTrigger asChild>
              <CirclePause height={12} width={12} />
            </TooltipTrigger>
            <TooltipContent>Paused</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <div className="ml-1">
          {(platform?.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor).containersPaused ?? '-'}
        </div>
      </div>
    );
  };

  const ContainersStarted = () => {
    return (
      <div className="flex items-center">
        <div className="text-green-500">
          <TooltipProvider delayDuration={200}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Power height={12} width={12} />
              </TooltipTrigger>
              <TooltipContent>Started</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <div className="ml-1">
          {(platform?.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor).containersRunning ?? '-'}
        </div>
      </div>
    );
  };

  const ContainersStopped = () => {
    return (
      <div className="flex items-center">
        <div className="text-red-500">
          <TooltipProvider delayDuration={200}>
            <Tooltip>
              <TooltipTrigger asChild>
                <PowerOff height={12} width={12} />
              </TooltipTrigger>
              <TooltipContent>Stopped</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <div className="ml-1">
          {(platform?.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor).containersStopped ?? '-'}
        </div>
      </div>
    );
  };

  const handleDeletePlatform = () => {
    setDialogData({ open: true, platform });
  };

  return (
    <div className="flow-root gap-1">
      <ul className="divide-y divide-foreground">
        <li className="group/platform py-3 bg-card/40 hover:bg-card/90 sm:py-4">
          <div className="flex flex-row flex-wrap items-center space-x-4">
            <div className="relative shrink-0">
              <div className="ml-1 w-20 h-20">
                <DockerIcon />
              </div>
              <LastSnapshotTooltip />
            </div>

            <div className="basis-5/12">
              <div className="flex items-baseline gap-2">
                <div className="cursor-pointer truncate text-sm font-medium hover:underline text-foreground">
                  <Link to={'/platforms/' + platform.id}>{platform.name}</Link>
                </div>
                <div className="truncate text-xs text-foreground">
                  ({(platform?.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor).operatingSystem} v
                  {platform?.serverVersion})
                </div>
              </div>

              <div className="flex flex-wrap gap-x-3">
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  <Link to={'/platforms/' + platform.id + '/containers'}>
                    {(platform?.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor).containerCount ?? '-'}{' '}
                    containers
                  </Link>
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  <Link to={'/platforms/' + platform.id + '/images'}>{platform?.imageCount ?? '-'} images</Link>
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  <Link to={'/platforms/' + platform.id + '/volumes'}>{platform?.volumeCount ?? '-'} volumes</Link>
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  <Link to={'/platforms/' + platform.id + '/networks'}>{platform?.networkCount ?? '-'} networks</Link>
                </div>
                <div className="truncate text-xs text-muted-foreground">{platform?.cpuCount ?? '-'} CPU</div>
                <div className="truncate text-xs text-muted-foreground">{byteTransform(platform?.memTotal)} RAM</div>
              </div>
            </div>
            <div className="flex flex-auto">
              <div>
                <div className="truncate text-center text-xs font-medium text-foreground">Containers</div>
                <div className="grid grid-cols-3 gap-4 truncate text-center text-xs text-gray-500 dark:text-night-400">
                  <ContainersStarted />
                  <ContainersStopped />
                  <ContainersPaused />
                </div>
              </div>
            </div>

            <div className="flex flex-auto">
              <div>
                <div className="truncate text-xs font-medium text-foreground">Memory usage</div>
                <div className="truncate text-center text-xs text-muted-foreground">
                  <span>
                    {platform.stats && isPlatfomOnline ? toFixedNumber(platform.stats[0]?.memoryUsage) + ' %' : 'N/A'}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex flex-auto">
              <div>
                <div className="truncate text-xs font-medium text-foreground">CPU usage</div>
                <div className="truncate text-center text-xs text-muted-foreground">
                  <span>
                    {platform.stats && isPlatfomOnline ? toFixedNumber(platform.stats[0]?.cpuUsage) + ' %' : 'N/A'}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex flex-auto items-center">
              <div className="inline-flex rounded-full invisible group-hover/platform:visible" role="group">
                <button
                  className="relative inline-flex items-center rounded-l-full border px-4 py-2 text-xs font-medium  hover:bg-foreground/10"
                  type="button">
                  <Pencil className="ml-1 h-3.5 w-3.5" />
                </button>
                <button
                  className="relative -ml-px inline-flex items-center rounded-r-full border px-4 py-2 text-xs font-medium text-red-700 dark:hover:bg-red-300 hover:bg-red-100"
                  onClick={handleDeletePlatform}
                  type="button">
                  <Trash2 className="mr-1 h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          </div>
        </li>
      </ul>
    </div>
  );
};

export default DockerPlatform;
