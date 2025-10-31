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
import { useCallback, useEffect, useLayoutEffect, useMemo, useState } from 'react';

import {
  AnyFn,
  ApiFn,
  ApiFnMap,
  DockerResourceType,
  KnownResourceName,
  PluralResourceMap,
  PrimaryArg,
  ResourceResponse,
  UseReadArgs,
} from '@/api/types';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { toast } from 'sonner';
import { useParams } from 'react-router';
import { useDeleteDialogAtom } from './atoms';
import { ProblemDetails } from '@/api/generated/api.types';

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

export function useMutate<
  TResource extends keyof ApiFnMap,
  TVariables = PrimaryArg<TResource> | { data: PrimaryArg<TResource>; params?: any },
>(
  resource: TResource,
  options?: Omit<UseMutationOptions<Awaited<ReturnType<ApiFn<TResource>>>, Error, TVariables>, 'mutationFn'>,
) {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const fn = apiClient.api[resource] as ApiFn<TResource>;

  const mutation = useMutation<Awaited<ReturnType<typeof fn>>, Error, TVariables>({
    mutationKey: [resource],
    ...options,
    mutationFn: ((variables: any) => {
      let args: any[] = [];

      const hasAllNamed =
        variables && typeof variables === 'object' && resDef.params.every((p: string) => p in variables);

      if (hasAllNamed) {
        args = resDef.params.map((param: string) => (param === 'params' ? (variables[param] ?? {}) : variables[param]));
      } else {
        if (variables && typeof variables === 'object' && 'data' in variables) {
          args = [variables.data, (variables as any).params ?? {}];
        } else {
          args = [variables];
          const fnParamLen = fn.length ?? 1;
          if (fnParamLen > 1) args.push({});
        }
      }
      return fn(...args);
    }) as MutationFunction<Awaited<ReturnType<typeof fn>>, TVariables>,
  });

  const validationErrors = useGetValidationErrors(mutation.error);

  return {
    ...mutation,
    validationErrors,
  } as UseMutationResult<Awaited<ReturnType<typeof fn>>, Error, TVariables, unknown> & {
    validationErrors: ReturnType<typeof useGetValidationErrors>;
  };
}

export function useDialogState<T>() {
  const [dialogData, setDialogData] = useState<IDialogData<T>>({ open: false });

  return { dialogData, setDialogData };
}

export interface IDialogData<T> {
  open: boolean;
  currentSelection?: T[];
}

interface DeleteDialogOptions {
  type: DockerResourceType;
  onSuccess?: () => void;
}

export function useDeleteDialog<T>({ type, onSuccess }: DeleteDialogOptions) {
  const { state, openDialog, closeDialog } = useDeleteDialogAtom<T>(type);
  const client = useQueryClient();

  const queryKeyToInvalidate = resources[`list${PluralResourceMap[type]}`].key;
  const mutationKey = resources[`delete${PluralResourceMap[type]}`].key;
  const resourceName = type.toLowerCase();

  const { mutate, isPending: deleteIsPending, isSuccess: deleteIsSuccess, error } = useMutate(mutationKey);

  use400ErrorToast(error, `The selected ${resourceName}(s) could not be deleted (status code: 400).`, closeDialog);

  useEffect(() => {
    if (deleteIsSuccess) {
      if (queryKeyToInvalidate) client.invalidateQueries({ queryKey: [queryKeyToInvalidate] });
      onSuccess?.();
      closeDialog();
      toast.success(`The selected ${resourceName}(s) have been successfully deleted`);
    }
  }, [deleteIsSuccess, client, queryKeyToInvalidate, resourceName, onSuccess, closeDialog]);

  const requestDelete = useCallback((data: any) => mutate(data), [mutate]);

  return {
    open: state.open,
    targets: state.targets as T[],
    openDialog,
    closeDialog,
    deleteIsPending,
    requestDelete,
  };
}

export const useDockerResourceParamType = () => {
  const type = useParams().type;
  if (!type) return undefined;
  return (type[0].toUpperCase() + type.slice(1, -1)) as DockerResourceType;
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

export const usePromptHotkeys = ({
  enabled = true,
  onConfirm,
  onCancel,
  ignoreInputs = true,
  confirmDisabled = false,
}: PromptHotkeysConfig) => {
  useEffect(() => {
    if (!enabled) return;

    const findConfirmButton = (): HTMLButtonElement | null => {
      const dialogContainers = document.querySelectorAll('[role="dialog"], [data-state="open"], .dialog-content');
      for (const container of dialogContainers) {
        const button = container.querySelector('[data-confirm-button]:not([disabled])') as HTMLButtonElement;
        if (button) return button;
      }

      return document.querySelector('[data-confirm-button]:not([disabled])') as HTMLButtonElement;
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (ignoreInputs) {
        const target = e.target as HTMLElement;
        if (
          target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.tagName === 'SELECT' ||
          target.isContentEditable
        ) {
          return;
        }
      }

      switch (e.key) {
        case 'Enter':
          if (onConfirm && !confirmDisabled) {
            e.preventDefault();
            const confirmButton = findConfirmButton();
            if (confirmButton) {
              confirmButton.click();
            } else {
              onConfirm();
            }
          }
          break;
        case 'Escape':
          if (onCancel) {
            e.preventDefault();
            onCancel();
          }
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [enabled, onConfirm, onCancel, ignoreInputs, confirmDisabled]);
};

export function useConfirmByName(args: {
  name: string;
  disabled?: boolean;
  onConfirm?: () => void | Promise<unknown>;
  onClose?: () => void;
  hotkeysEnabled?: boolean;
}) {
  const { name, disabled, onConfirm, onClose, hotkeysEnabled } = args;
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

  usePromptHotkeys({
    onConfirm: () => {
      if (name === input && !disabled && !isLoading) handleConfirm();
    },
    onCancel: () => onClose?.(),
    enabled: !!hotkeysEnabled,
    confirmDisabled: !!disabled || name !== input || isLoading,
  });

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
      if (error.status != null && (error.status as number) > 400) {
        toast.error(error.status + ' ' + error.title, {
          description: error.detail,
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
