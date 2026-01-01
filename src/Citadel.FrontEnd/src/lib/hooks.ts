import { resources } from '@/api/generated/resources';
import {
  MutationFunction,
  useMutation,
  UseMutationOptions,
  UseMutationResult,
  useQuery,
  useQueryClient,
  UseQueryOptions,
  UseQueryResult,
} from '@tanstack/react-query';
import { useApiClientContext } from '@/api/api-client-context';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';

import {
  AnyFn,
  ApiFn,
  Cancellable,
  KnownResourceName,
  ResourceResponse,
  ResourceType,
  UseMutateVariables,
  UseReadArgs,
} from '@/api/types';
import { useGetValidationErrors, getValidationErrors } from '@/hooks/useGetValidationErrors';
import { toast } from 'sonner';
import { useParams } from 'react-router';
import { ApplyDeploymentInput, ProblemDetails, PullImageInput } from '@/api/generated/api.types';
import { useAuthContext } from '@/features/auth/auth-context';

const EMPTY_ARGS = Object.freeze({});

export function useRead<
  TResource extends KnownResourceName,
  TResult extends ResourceResponse<TResource> = ResourceResponse<TResource>,
>(
  resource: TResource,
  args?: UseReadArgs<TResource>,
  options?: Omit<
    UseQueryOptions<TResult, Error, TResult, readonly [TResource, UseReadArgs<TResource> | {}]>,
    'queryKey' | 'queryFn'
  >,
): UseQueryResult<TResult, Error> {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const stableArgs = args ?? EMPTY_ARGS;
  const queryKey = useMemo(() => [resource, stableArgs] as const, [resource, stableArgs]);

  const isEnabled = !!apiClient && resDef.requiredParams.every((p) => (args as any)?.[p] != null);

  return useQuery<TResult, Error, TResult, readonly [TResource, UseReadArgs<TResource> | {}]>({
    queryKey,
    enabled: options?.enabled ?? isEnabled,
    queryFn: async ({ signal }) => {
      const fn = (apiClient?.api as any)[resource] as AnyFn;
      if (!fn) throw new Error(`Unknown API resource: ${resource}`);

      // Build arguments dynamically based on the resource definition
      const callArgs = resDef.params.map((paramName) => {
        if (paramName === 'params') return { ...(args?.params ?? {}), signal };
        if (paramName === 'query') return args?.query ?? {};
        return (args as any)?.[paramName];
      });

      return fn(...callArgs);
    },
    ...options,
  });
}

export function useMutate<TResource extends KnownResourceName, TVariables = UseMutateVariables<TResource>>(
  resource: TResource,
  options?: Omit<UseMutationOptions<ResourceResponse<TResource>, Error, TVariables>, 'mutationFn'>,
) {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const fn = apiClient.api[resource] as ApiFn<TResource>;

  const mutation = useMutation<ResourceResponse<TResource>, Error, TVariables>({
    mutationKey: [resource],
    ...options,
    mutationFn: ((variables: TVariables) => {
      const v: any = variables;
      const paramNames = resDef.params ?? [];
      const requiredParamNames = resDef.requiredParams ?? [];
      const args: any[] = [];
      const isObject = v !== null && typeof v === 'object';

      const hasAllRequiredNamed =
        isObject && requiredParamNames.length > 0 && requiredParamNames.every((p: string) => p in v);

      if (hasAllRequiredNamed) {
        for (const name of paramNames) {
          if (name === 'params') {
            args.push(v.params ?? {});
          } else {
            args.push(v[name]);
          }
        }
      } else if (isObject && 'data' in v) {
        args.push(v.data);
        args.push(v.params ?? {});
      } else {
        args.push(v);
        const fnParamLen = fn.length ?? 1;
        if (fnParamLen > 1) {
          args.push({});
        }
      }

      return fn(...args);
    }) as MutationFunction<ResourceResponse<TResource>, TVariables>,
  });

  const validationErrors = useGetValidationErrors(mutation.error);

  return {
    ...mutation,
    validationErrors,
  } as UseMutationResult<ResourceResponse<TResource>, Error, TVariables, unknown> & {
    validationErrors: ReturnType<typeof useGetValidationErrors>;
  };
}

export const useResourceParamType = (): ResourceType => {
  const type = useParams().type;
  if (type === 'registries') return 'Registry';
  return type ? ((type[0].toUpperCase() + type.slice(1, -1)) as ResourceType) : 'Platform';
};

export function useLocalStorage<T>(key: string, initialValue: T) {
  const [value, setValue] = useState<T>(() => {
    try {
      const stored = localStorage.getItem(key);
      return stored ? (JSON.parse(stored) as T) : initialValue;
    } catch {
      return initialValue;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      /** Ignore */
    }
  }, [key, value]);

  return [value, setValue] as const;
}

export function useConfirmByName(args: {
  name: string;
  disabled?: boolean;
  onConfirm?: () => void | Promise<unknown>;
  onClose?: () => void;
  hotkeysEnabled?: boolean;
}) {
  const { name, disabled, onConfirm, onClose } = args;
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleConfirm = useCallback(() => {
    try {
      setIsLoading(true);
      const maybe = onConfirm?.();
      Promise.resolve(maybe)
        .then(() => onClose?.())
        .catch((err) => {
          const problem = (err as any)?.error as ProblemDetails;
          if (problem && problem.status === 400) {
            toast.error(`400: ${problem.title ?? 'Bad Request'}`, { description: problem?.detail });
          }
        })
        .finally(() => setIsLoading(false));
    } catch {
      setIsLoading(false);
    }
  }, [onConfirm, onClose]);

  const isConfirmDisabled = !!disabled || name !== input || isLoading;
  const reset = () => setInput('');

  return { input, setInput, isLoading, isConfirmDisabled, handleConfirm, reset } as const;
}

export interface PromptHotkeysConfig {
  onConfirm?: () => void;
  onCancel?: () => void;
  enabled?: boolean;
  ignoreInputs?: boolean;
  confirmDisabled?: boolean;
}

export function useStickySentinel(topOffsetPx: number = 0) {
  const [isStuck, setIsStuck] = useState(false);
  const [node, setNode] = useState<HTMLDivElement | null>(null);

  const sentinelRef = useCallback((el: HTMLDivElement | null) => {
    setNode(el);
  }, []);

  useLayoutEffect(() => {
    if (!node) return;
    const rect = node.getBoundingClientRect();
    setIsStuck(rect.top <= topOffsetPx);
  }, [topOffsetPx, node]);

  useEffect(() => {
    if (!node) return;
    const observer = new IntersectionObserver(([entry]) => setIsStuck(!entry.isIntersecting), {
      root: null,
      threshold: 1,
      rootMargin: `-${topOffsetPx}px 0px 0px 0px`,
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, [topOffsetPx, node]);

  return { sentinelRef, isStuck } as const;
}

export function useHTTPErrorHandler() {
  const client = useQueryClient();

  useEffect(() => {
    const handleError = (error: ProblemDetails) => {
      if (!error) return;
      if (error.status != null) {
        if (error.status === 401) return;
        toast.error(error.status + ' ' + error.title, {
          description: error.status === 400 ? getValidationErrors(error) : error.detail,
        });
      }
    };
    const mutationUnsubscribe = client.getMutationCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error.error);
      }
    });

    const queryUnsubscribe = client.getQueryCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error.error);
      }
    });

    return () => {
      mutationUnsubscribe();
      queryUnsubscribe();
    };
  }, [client]);
}
interface DialogHotkeysOptions {
  enabled?: boolean;
  confirmDisabled?: boolean;
  onConfirm?: () => void;
  onCancel?: () => void;
  confirmButtonRef?: React.RefObject<HTMLButtonElement | null>;
}

export function useDialogHotkeys({
  enabled = true,
  confirmButtonRef,
  onConfirm,
  onCancel,
  confirmDisabled = false,
}: DialogHotkeysOptions) {
  useEffect(() => {
    if (!enabled) return;

    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        if (!confirmDisabled) {
          e.preventDefault();
          if (confirmButtonRef?.current) {
            confirmButtonRef.current.click();
          } else {
            onConfirm?.();
          }
        }
      }

      if (e.key === 'Escape') {
        e.preventDefault();
        onCancel?.();
      }
    };

    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [enabled, confirmDisabled, onConfirm, onCancel, confirmButtonRef]);
}

export function useMeasuredWidth(max = 400) {
  const ref = useRef<HTMLElement | null>(null);
  const [width, setWidth] = useState<number | undefined>(undefined);

  const measure = useCallback(() => {
    const w = ref.current?.getBoundingClientRect().width;
    setWidth(w ? Math.min(w, max) : undefined);
  }, [max]);

  useLayoutEffect(() => {
    measure();
  }, [measure]);

  useEffect(() => {
    const onResize = () => measure();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [measure]);

  return { ref, width, measure };
}

export type Dimensions = { width: number; height: number };
export const useWindowDimensions = () => {
  const [dimensions, setDimensions] = useState<Dimensions>({
    width: 0,
    height: 0,
  });
  useEffect(() => {
    const callback = () => {
      setDimensions({
        width: window.innerWidth,
        height: window.innerHeight,
      });
    };
    callback();
    window.addEventListener('resize', callback);
    return () => {
      window.removeEventListener('resize', callback);
    };
  }, []);
  return dimensions;
};

type PulledStreamProps = PullImageInput | ApplyDeploymentInput;

const usePulledStream = (onChunkReceived: (chunk: string) => void, endpoint: string) => {
  const { apiClient } = useApiClientContext();
  const { accessToken } = useAuthContext();

  const mutationFn = async (param: PulledStreamProps & Cancellable) => {
    if (!apiClient?.baseUrl) {
      throw new Error('API client base URL is not defined');
    }

    const response = await fetch(`${apiClient.baseUrl}/${endpoint}`, {
      method: 'POST',
      credentials: 'include',
      signal: param.signal,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify(param),
    });

    if (!response.ok) {
      throw new Error(`Network response was not ok: ${response.status} ${response.statusText}`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('ReadableStream is not supported or response body is null');
    }

    const decoder = new TextDecoder('utf-8');
    let done = false;

    try {
      while (!done) {
        const { value, done: readerDone } = await reader.read();
        done = readerDone;

        if (value) {
          const chunk = decoder.decode(value, { stream: true });
          onChunkReceived(chunk);
        }
      }
    } catch (error) {
      console.error('Error while reading stream:', error);
      throw error;
    } finally {
      reader.releaseLock();
    }
  };

  const { mutate, isPending, isSuccess, error } = useMutation({ mutationFn });

  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, error, validationErrors };
};

type StreamStatus = 'pending' | 'success' | 'error';

interface StreamProgressState {
  lines: string[];
  text: string;
  isPending: boolean;
  isSuccess: boolean;
  status: StreamStatus;
  error?: string;
  elapsedMs: number;
  elapsedLabel: string;
}

interface UseStreamProgressOptions<TRequest, TItem> {
  endpoint: string;
  request: TRequest;
  successMessage: string;
  errorMessageDefault: string;
  // A predicate to check if an item in the stream represents an error
  getError?: (item: TItem) => string | undefined | null;
}

export function useStreamProgress<TRequest extends PulledStreamProps, TItem>({
  endpoint,
  request,
  successMessage,
  errorMessageDefault,
  getError,
}: UseStreamProgressOptions<TRequest, TItem>): StreamProgressState {
  const [lines, setLines] = useState<string[]>([]);
  const [internalError, setInternalError] = useState<string | undefined>();
  const abortControllerRef = useRef<AbortController | null>(null);

  const [elapsedMs, setElapsedMs] = useState<number>(0);
  const startRef = useRef<number | null>(null);
  const timerRef = useRef<number | null>(null);

  const handleChunkReceived = useCallback((chunk: string) => {
    setLines((prev) => [...prev, chunk]);
  }, []);

  const { isPending, isSuccess, error: streamError, mutate } = usePulledStream(handleChunkReceived, endpoint);

  // Handle Request Lifecycle
  useEffect(() => {
    const controller = new AbortController();
    abortControllerRef.current = controller;

    mutate({ ...request, signal: controller.signal });

    return () => {
      controller.abort();
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [mutate, request]);

  // Handle Success/Error Parsing
  useEffect(() => {
    if (isSuccess) {
      try {
        const data = lines.join('\n');
        const response = JSON.parse(data) as TItem[];

        const errors = getError ? response.map(getError).filter(Boolean) : [];

        if (errors.length > 0) {
          const err = (errors[0] as string) || errorMessageDefault;
          setInternalError(err);
          toast.error('Error', { description: err });
        } else {
          toast.success(successMessage);
        }
      } catch {
        const msg = 'Failed to parse response';
        setInternalError(msg);
        toast.error('Error', { description: msg });
      }
    }
  }, [isSuccess, lines, getError, successMessage, errorMessageDefault]);

  // Handle Timer
  useEffect(() => {
    if (isPending) {
      if (startRef.current == null) startRef.current = performance.now();
      timerRef.current = window.setInterval(() => {
        setElapsedMs(Math.max(0, performance.now() - (startRef.current ?? 0)));
      }, 100);
    } else if (timerRef.current) {
      clearInterval(timerRef.current);
    }
  }, [isPending]);

  const status: StreamStatus = useMemo(() => {
    if (isPending) return 'pending';
    if (internalError || streamError) return 'error';
    return 'success';
  }, [isPending, internalError, streamError]);

  const elapsedLabel = useMemo(() => {
    return (Math.floor(elapsedMs / 100) / 10).toFixed(1).replace('.', ',');
  }, [elapsedMs]);

  return {
    lines,
    text: lines.length === 0 && isPending ? 'Loading...' : lines.join('\n'),
    isPending,
    isSuccess,
    status,
    error: internalError || (streamError as any)?.message,
    elapsedMs,
    elapsedLabel,
  };
}
