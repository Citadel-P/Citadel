import { ResourceType } from '@/api/types';
import { atom, PrimitiveAtom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { TaskSpec, TaskSheetState } from '@/components/custom/task-sheet';
import { useCallback, useEffect } from 'react';
import { useSearchParams } from 'react-router';
import { ActivityEventType, ActivityResourceType } from '@/api/generated/api.types';

const segmentTitleAtom = atom<{ action: string; name: string } | null>(null);
const taskSheetAtom = atomFamily((_: ResourceType) => atom<TaskSheetState>({ open: false }));
const selectedResourcesAtoms = atomFamily((_: string) => atom<any[]>([]));
const resourceFilterAtom = atomFamily((_: ResourceType) => atom<{ item: any } | null>(null));
const inlineSubHeaderAtom = atomFamily((_: ResourceType) => atom<boolean>(false));

export type PagingKey = ResourceType | string;

export type ActivityQueryState = ActivityFiltersState & { page: number; pageSize: number };

const activityQueryAtom = atom<ActivityQueryState>({
  resourceType: 'All',
  eventType: 'All',
  resourceId: undefined,
  page: 1,
  pageSize: 50,
});

export function useActivityQuery() {
  const [state, setState] = useAtomUrlSync<ActivityQueryState>({
    atom: activityQueryAtom,
    params: ['resourceType', 'eventType', 'resourceId', 'page', 'pageSize'],
    defaults: { resourceType: 'All', eventType: 'All', resourceId: undefined, page: 1, pageSize: 50 },
    serialize: (v) => (v === 'All' ? null : String(v)),
    deserialize: (raw, key) => {
      if (key === 'page' || key === 'pageSize') {
        const n = Number(raw);
        return Number.isFinite(n) && n > 0 ? n : key === 'page' ? 1 : 50;
      }
      return (raw ?? 'All') as any;
    },
  });

  const setQuery = useCallback(
    (patch: Partial<ActivityQueryState>) => {
      setState((prev) => ({ ...prev, ...patch }));
    },
    [setState],
  );

  return [state, setQuery] as const;
}

export type PagingState = {
  page: number;
  pageSize: number;
};

export type ActivityFiltersState = {
  resourceType: ActivityResourceType | 'All';
  eventType: ActivityEventType | 'All';
  resourceId: string | undefined;
};

export function useSegmentTitle() {
  const [segmentTitle, setSegmentTitle] = useAtom(segmentTitleAtom);
  return [segmentTitle, setSegmentTitle] as const;
}

export function useSelectedResources<T>(key: ResourceType) {
  const [selected, setSelected] = useAtom(selectedResourcesAtoms(key));
  return [selected as T[], setSelected as (items: T[]) => void] as const;
}

export function useResourceFilter<T extends object = any>(key: ResourceType) {
  const [filter, setFilter] = useAtom(resourceFilterAtom(key));
  return [filter as T | null, setFilter as (value: T | null) => void] as const;
}

export function useTaskSheet(type: ResourceType) {
  const [state, setState] = useAtom(taskSheetAtom(type));

  const open = (task: TaskSpec) => setState({ open: true, task });
  const close = () => setState((s) => ({ ...s, open: false }));

  return { state, open, close } as const;
}

export function useInlineSubHeader(type: ResourceType) {
  const [open, setOpen] = useAtom(inlineSubHeaderAtom(type));
  const show = () => setOpen(true);
  const hide = () => setOpen(false);
  const toggle = () => setOpen((v) => !v);
  return { open, show, hide, toggle } as const;
}

type UrlSyncConfig<T> = {
  atom: PrimitiveAtom<T>;
  params: (keyof T)[];
  serialize: (value: T[keyof T], key: keyof T) => string | null;
  deserialize: (raw: string | null, key: keyof T) => T[keyof T];
  defaults: T;
};

function useAtomUrlSync<T extends Record<string, any>>(config: UrlSyncConfig<T>) {
  const { atom, params, serialize, deserialize, defaults } = config;
  const [state, setState] = useAtom(atom);
  const [searchParams, setSearchParams] = useSearchParams();

  useEffect(() => {
    const next: Partial<T> = {};
    let hasChanged = false;

    for (const key of params) {
      const raw = searchParams.get(String(key));
      const value = deserialize(raw, key);
      if (value !== state[key]) {
        next[key] = value;
        hasChanged = true;
      }
    }

    if (hasChanged) {
      setState((prev) => ({ ...prev, ...next }));
    }
  }, [searchParams]);

  const setSyncedState = useCallback(
    (nextValOrUpdater: T | ((prev: T) => T)) => {
      setState((prev) => {
        const next = typeof nextValOrUpdater === 'function' ? (nextValOrUpdater as any)(prev) : nextValOrUpdater;

        const nextParams = new URLSearchParams(window.location.search);
        params.forEach((key) => {
          const serialized = serialize(next[key], key);
          if (serialized === null || next[key] === defaults[key]) {
            nextParams.delete(String(key));
          } else {
            nextParams.set(String(key), serialized);
          }
        });

        if (nextParams.toString() !== searchParams.toString()) {
          setSearchParams(nextParams, { replace: true });
        }

        return next;
      });
    },
    [params, serialize, defaults, searchParams, setSearchParams, setState],
  );

  return [state, setSyncedState] as const;
}
