import type { ResourceType } from '@/api/types';
import { useSelectedResources } from '@/lib/atoms';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { useEffect, useRef, useState } from 'react';

async function settleResources<T>(items: T[], mutate: (item: T) => Promise<unknown>) {
  const outcomes = await Promise.allSettled(items.map(async (item) => mutate(item)));
  return items.filter((_, index) => outcomes[index].status === 'fulfilled');
}

export function useSettledResourceAction<T extends { id: string }>(
  items: T[],
  mutate: (item: T) => Promise<unknown>,
  completed: (succeeded: T[]) => void | Promise<unknown>,
) {
  const running = useRef(false);
  const [isPending, setPending] = useState(false);
  return {
    canExecute: items.length > 0,
    isPending,
    run: async () => {
      if (running.current || !items.length) return;
      running.current = true;
      setPending(true);
      try {
        await completed(await settleResources(items, mutate));
      } finally {
        running.current = false;
        setPending(false);
      }
    },
  };
}

export function useRemoveResourceAction<T extends { id: string }>(
  items: T[],
  options: {
    type: ResourceType;
    listPath: string;
    mutate: (item: T) => Promise<unknown>;
    onRemoved?: () => void | Promise<unknown>;
    message: (success: number, total: number) => string;
  },
) {
  const [, setSelected] = useSelectedResources<T>(options.type);
  const location = useLocation();
  const navigate = useNavigate();
  const path = useRef<string | null>(null);
  useEffect(() => {
    path.current = location.pathname.replace(/\/$/, '');
    return () => {
      path.current = null;
    };
  }, [location.pathname]);
  return useSettledResourceAction(items, options.mutate, async (succeeded) => {
    const ids = new Set(succeeded.map((item) => item.id));
    setSelected((current) => current.filter((item) => !ids.has(item.id)));
    if (succeeded.some((item) => path.current === `${options.listPath}/edit/${item.id}`))
      navigate(options.listPath, { replace: true });
    if (ids.size) {
      await options.onRemoved?.();
      toast.success(options.message(ids.size, items.length));
    }
  });
}
