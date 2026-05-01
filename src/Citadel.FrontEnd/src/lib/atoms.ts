import { ResourceType } from '@/api/types';
import { atom, PrimitiveAtom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { TaskSpec, TaskSheetState } from '@/components/custom/task-sheet';
import { useCallback, useEffect, useRef } from 'react';
import { useSearchParams } from 'react-router';
import { ActivityEventType, ActivityResourceType, AlertResourceType, AlertType } from '@/api/generated/api.types';

const segmentTitleAtom = atom<{ action: string; name: string } | null>(null);
const taskSheetAtom = atomFamily((_: ResourceType) => atom<TaskSheetState>({ open: false }));
const selectedResourcesAtoms = atomFamily((_: string) => atom<any[]>([]));
const resourceFilterAtom = atomFamily((_: ResourceType) => atom<{ item: any } | null>(null));
const inlineSubHeaderAtom = atomFamily((_: ResourceType) => atom<boolean>(false));

export type PagingKey = ResourceType | string;

export type UserQueryState = { userName: string | undefined; page: number; pageSize: number };
export type ActivityQueryState = ActivityFiltersState & { page: number; pageSize: number };
export type AlertEventQueryState = AlertEventFiltersState & { page: number; pageSize: number };

const userQueryAtom = atom<UserQueryState>({ userName: '', page: 1, pageSize: 50 });
const activityQueryAtom = atom<ActivityQueryState>({
  resourceType: 'All',
  eventType: 'All',
  resourceId: undefined,
  page: 1,
  pageSize: 50,
});

const alertEventQueryAtom = atom<AlertEventQueryState>({
  resourceType: 'All',
  alertType: 'All',
  resourceId: undefined,
  unresolvedOnly: false,
  page: 1,
  pageSize: 50,
});

export function useActivityQuery() {
  const [state, setState] = useAtomUrlSync<ActivityQueryState>({
    atom: activityQueryAtom,
    params: ['resourceType', 'eventType', 'resourceId', 'page', 'pageSize'],
    defaults: { resourceType: 'All', eventType: 'All', resourceId: undefined, page: 1, pageSize: 50 },
    serialize: (v, key) => {
      if (key === 'resourceId') {
        return v ? String(v) : null;
      }
      return v === 'All' ? null : String(v);
    },
    deserialize: (raw, key) => {
      if (key === 'page' || key === 'pageSize') {
        const n = Number(raw);
        return Number.isFinite(n) && n > 0 ? n : key === 'page' ? 1 : 50;
      }
      if (key === 'resourceId') {
        return raw ?? undefined;
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

export function useUserQuery() {
  const [state, setState] = useAtomUrlSync<UserQueryState>({
    atom: userQueryAtom,
    params: ['userName', 'page', 'pageSize'],
    defaults: { userName: undefined, page: 1, pageSize: 50 },
    serialize: (v, key) => {
      if (key === 'userName') {
        return v ? String(v) : null;
      }
      return null;
    },
    deserialize: (raw, key) => {
      if (key === 'page' || key === 'pageSize') {
        const n = Number(raw);
        return Number.isFinite(n) && n > 0 ? n : key === 'page' ? 1 : 50;
      }
      if (key === 'userName') {
        return raw ?? undefined;
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

export function useAlertEventQuery() {
  const [state, setState] = useAtomUrlSync<AlertEventQueryState>({
    atom: alertEventQueryAtom,
    params: ['resourceType', 'alertType', 'resourceId', 'unresolvedOnly', 'page', 'pageSize'],
    defaults: {
      resourceType: 'All',
      alertType: 'All',
      resourceId: undefined,
      unresolvedOnly: false,
      page: 1,
      pageSize: 50,
    },
    serialize: (v, key) => {
      if (key === 'unresolvedOnly') {
        return v === true ? 'true' : null;
      }
      if (key === 'resourceId') {
        return v ? String(v) : null;
      }
      return v === 'All' ? null : String(v);
    },
    deserialize: (raw, key) => {
      if (key === 'page' || key === 'pageSize') {
        const n = Number(raw);
        return Number.isFinite(n) && n > 0 ? n : key === 'page' ? 1 : 50;
      }
      if (key === 'unresolvedOnly') {
        return raw === 'true';
      }
      if (key === 'resourceId') {
        return raw ?? undefined;
      }
      return (raw ?? 'All') as any;
    },
  });

  const setQuery = useCallback(
    (patch: Partial<AlertEventQueryState>) => {
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

export type AlertEventFiltersState = {
  alertType: AlertType | 'All';
  resourceId: string | undefined;
  unresolvedOnly: boolean | undefined;
  resourceType: AlertResourceType | 'All';
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

  const deserializeRef = useRef(deserialize);
  deserializeRef.current = deserialize;
  const paramsRef = useRef(params);
  paramsRef.current = params;
  const serializeRef = useRef(serialize);
  serializeRef.current = serialize;
  const defaultsRef = useRef(defaults);
  defaultsRef.current = defaults;

  useEffect(() => {
    setState((prev) => {
      const next: Partial<T> = {};
      let hasChanged = false;

      for (const key of paramsRef.current) {
        const raw = searchParams.get(String(key));
        const value = deserializeRef.current(raw, key);
        if (value !== prev[key]) {
          next[key] = value as any;
          hasChanged = true;
        }
      }

      return hasChanged ? { ...prev, ...next } : prev;
    });
  }, [searchParams, setState]);

  const setSyncedState = useCallback(
    (nextValOrUpdater: T | ((prev: T) => T)) => {
      setState((prev) => {
        const next =
          typeof nextValOrUpdater === 'function' ? (nextValOrUpdater as any)(prev) : { ...prev, ...nextValOrUpdater };

        const nextParams = new URLSearchParams(window.location.search);
        paramsRef.current.forEach((key) => {
          const serialized = serializeRef.current(next[key], key);
          if (serialized === null || next[key] === defaultsRef.current[key]) {
            nextParams.delete(String(key));
          } else {
            nextParams.set(String(key), serialized);
          }
        });

        if (nextParams.toString() !== new URLSearchParams(window.location.search).toString()) {
          setSearchParams(nextParams, { replace: true });
        }

        return next;
      });
    },
    [setSearchParams, setState],
  );

  return [state, setSyncedState] as const;
}
