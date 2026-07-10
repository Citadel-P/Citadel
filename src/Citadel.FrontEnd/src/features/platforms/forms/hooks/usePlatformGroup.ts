import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { normalizePlatform } from '../../hooks/usePlatformsGroup';

export const usePlatformGroup = (id: string) => {
  const args = useMemo(() => ({ id }), [id]);
  const { data, isLoading } = useRead('getPlatfom', args, { enabled: Boolean(id) });
  const platform = data?.data ? normalizePlatform(data.data) : undefined;

  return { platform, isLoading };
};
