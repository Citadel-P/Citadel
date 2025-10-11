import { resources } from '@/api/generated/resources';
import {
  MutationFunction,
  useMutation,
  UseMutationOptions,
  UseMutationResult,
  useQuery,
  UseQueryOptions,
  UseQueryResult,
} from '@tanstack/react-query';
import { useApiClientContext } from '@/api/ApiClientContext';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { KnownResourceName, ResourceResponse, UseReadArgs } from '@/api/resource-types';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

type AnyFn = (...args: any[]) => Promise<any>;
const EMPTY_ARGS = Object.freeze({});

export function useRead<
  TResource extends KnownResourceName,
  TResult extends ResourceResponse<TResource> = ResourceResponse<TResource>,
>(
  resource: TResource,
  args?: UseReadArgs<TResource>,
  options?: Omit<
    UseQueryOptions<TResult, Error, TResult, readonly [TResource, {} | UseReadArgs<TResource>]>,
    'queryKey' | 'queryFn' | 'enabled'
  >,
): UseQueryResult<TResult, Error> {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const queryObj: Record<string, any> = args?.query ?? {};

  const stableArgs = args ?? EMPTY_ARGS;
  const queryKey = useMemo(() => [resource, stableArgs] as const, [resource, stableArgs]);

  const hasMounted = useRef(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    hasMounted.current = true;
    setMounted(true);
  }, []);

  const isEnabled = !!apiClient && resDef.requiredParams.every((p) => (args as any)?.[p] != null) && mounted;

  return useQuery<TResult, Error, TResult, readonly [TResource, {} | UseReadArgs<TResource>]>({
    queryKey,
    enabled: isEnabled,
    queryFn: async ({ signal }) => {
      const fn = (apiClient.api as any)[resource] as AnyFn;
      if (!fn) throw new Error(`Unknown API resource: ${resource}`);

      const callArgs = resDef.params.map((paramName) => {
        if (paramName === 'params') return { ...(args?.params ?? {}), signal };
        if (paramName === 'query') return queryObj;
        return (args as any)?.[paramName] ?? undefined;
      });

      callArgs.push({ signal });
      return fn(...callArgs);
    },
    ...options,
  });
}

const WRITE_METHODS = ['POST', 'PATCH', 'PUT', 'DELETE'] as const;

// Infer API client method types
type ApiClientType = ReturnType<typeof useApiClientContext>['apiClient'];
type ApiFnMap = ApiClientType['api'];
type ApiFn<TResource extends keyof ApiFnMap> = ApiFnMap[TResource];

type MutateVariables<TResource extends keyof typeof resources> = {
  [K in (typeof resources)[TResource]['requiredParams'][number]]: string;
} & {
  [K in Exclude<
    (typeof resources)[TResource]['params'][number],
    (typeof resources)[TResource]['requiredParams'][number]
  >]?: any;
};

export function useMutate<TResource extends keyof ApiFnMap>(
  resource: TResource,
  options?: Omit<
    UseMutationOptions<Awaited<ReturnType<ApiFn<TResource>>>, Error, MutateVariables<TResource>>,
    'mutationFn'
  >,
) {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource as keyof typeof resources];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const fn = apiClient.api[resource] as ApiFn<TResource>;
  if (!WRITE_METHODS.includes(resDef.method as (typeof WRITE_METHODS)[number])) {
    throw new Error(`useMutate can only be used with write endpoints, got ${resDef.method}`);
  }

  const mutation = useMutation<Awaited<ReturnType<typeof fn>>, Error, MutateVariables<TResource>>({
    ...options,
    mutationFn: ((variables: MutateVariables<TResource>) => {
      const args = resDef.params.map((param) => variables[param as keyof MutateVariables<TResource>]);
      return fn(...args);
    }) as unknown as MutationFunction<Awaited<ReturnType<typeof fn>>, MutateVariables<TResource>>,
  });

  const validationErrors = useGetValidationErrors(mutation.error);

  return { ...mutation, validationErrors } as UseMutationResult<
    Awaited<ReturnType<typeof fn>>,
    Error,
    MutateVariables<TResource>,
    unknown
  > & { validationErrors: ReturnType<typeof useGetValidationErrors> };
}

export type LocalStorageSetter<T> = (state: T) => T;

export const useLocalStorage = <T>(key: string, init: T): [T, (state: T | LocalStorageSetter<T>) => void] => {
  const stored = localStorage.getItem(key);
  const parsed = stored ? (JSON.parse(stored) as T) : undefined;
  const [state, inner_set] = useState<T>(parsed ?? init);
  const set = (state: T | LocalStorageSetter<T>) => {
    inner_set((prev_state) => {
      const new_val = typeof state === 'function' ? (state as LocalStorageSetter<T>)(prev_state) : state;
      localStorage.setItem(key, JSON.stringify(new_val));
      return new_val;
    });
  };
  return [state, set];
};
