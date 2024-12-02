import { PlatformView } from '@/api/_generated';
import DockerIcon from '@/assets/docker.svg';
import { Link } from 'react-router';
import { Power, PowerOff, CirclePause, ChevronRight } from 'lucide-react';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import useProbeCheck from './hooks/useProbeCheck';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { fromNow } from '@/lib/dayjs.helper';

interface IProps {
  platform: PlatformView;
}

const Platform = ({ platform }: IProps) => {
  const { isProbActive, lastSnapshot } = useProbeCheck(platform);

  const LastSnapshotTooltip = () => {
    return (
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div
              className={`absolute right-2 top-1.5 ${isProbActive ? 'bg-green-500' : 'bg-red-500'} h-3.5 w-3.5 rounded-full border-2`}></div>
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
        <div className="ml-1">{platform.systemInfo?.containersPaused}</div>
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
        <div className="ml-1">{platform.systemInfo?.containersRunning}</div>
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
        <div className="ml-1">{platform.systemInfo?.containersStopped}</div>
      </div>
    );
  };

  return (
    <div className="flow-root mb-1">
      <ul className="divide-y divide-foreground">
        <li className="group/platform py-3 bg-card/40 hover:bg-card/90 sm:py-4">
          <div className="flex flex-row flex-wrap items-center space-x-4">
            <div className="relative flex-shrink-0">
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
                  ({platform.systemInfo?.operatingSystem} v{platform.systemInfo?.serverVersion})
                </div>
              </div>

              <div className="flex flex-wrap gap-x-3">
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  <Link to={'/platforms/' + platform.id + '/containers'}>
                    {platform.systemInfo?.containers} containers
                  </Link>
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  {platform.systemInfo?.images} images
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  {platform.systemInfo?.volumesCount} volumes
                </div>
                <div className="cursor-pointer truncate text-xs text-muted-foreground hover:underline">
                  {platform.systemInfo?.networksCount} networks
                </div>
                <div className="truncate text-xs text-muted-foreground">{platform.systemInfo?.ncpu} CPU</div>
                <div className="truncate text-xs text-muted-foreground">
                  {byteTransform(platform.systemInfo?.memTotal)} RAM
                </div>
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
                    {platform.stats && isProbActive ? toFixedNumber(platform.stats[0]?.memoryUsage) + ' %' : 'N/A'}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex flex-auto">
              <div>
                <div className="truncate text-xs font-medium text-foreground">CPU usage</div>
                <div className="truncate text-center text-xs text-muted-foreground">
                  <span>
                    {platform.stats && isProbActive ? toFixedNumber(platform.stats[0]?.cpuUsage) + ' %' : 'N/A'}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex flex-auto">
              <button className="group/edit invisible flex items-center truncate rounded-full p-2 text-xs font-medium group-hover/platform:visible hover:bg-foreground/10">
                <span>Edit</span>
                <ChevronRight className="ml-1 h-3.5 w-3.5 group-hover/edit:translate-x-0.5" />
              </button>
            </div>
          </div>
        </li>
      </ul>
    </div>
  );
};

export default Platform;
