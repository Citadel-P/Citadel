import { PlatformView } from '@/api/_generated';
import useInterval from '@/hooks/useInterval';
import { useState } from 'react';

const useProbeCheck = (platform: PlatformView) => {
  const [isProbActive, setIsProbActive] = useState(false);

  const lastSnapshot =
    platform.stats && platform.stats[0] ? new Date(platform.stats![0].created! * 1000).getTime() : new Date().getTime();

  const updateProbe = () => {
    /** If we do not receive any platform stats within 60s then we consider the agent as disconnected */
    const snapShotValid = new Date().getTime() - (lastSnapshot + 60000) < 0;
    setIsProbActive(snapShotValid);
  };

  useInterval(
    () => {
      updateProbe();
    },
    5000,
    true,
  );

  return { isProbActive, lastSnapshot };
};

export default useProbeCheck;
