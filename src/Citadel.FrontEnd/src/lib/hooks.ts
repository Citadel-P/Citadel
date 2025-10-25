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
import { useApiClientContext } from '@/api/ApiClientContext';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  AnyFn,
  ApiFn,
  ApiFnMap,
  DockerResourceType,
  KnownResourceName,
  MutateVariables,
  PluralResourceMap,
  ResourceResponse,
  UseReadArgs,
} from '@/api/types';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { toast } from 'sonner';
import { useParams } from 'react-router';
import { useDeleteDialogAtom } from './atoms';

const EMPTY_ARGS = Object.freeze({});

export function useRead<
  TResource extends KnownResourceName,
  TResult extends ResourceResponse<TResource> = ResourceResponse<TResource>,
>(
  resource: TResource,
  args?: UseReadArgs<TResource>,
  options?: Omit<
    UseQueryOptions<TResult, Error, TResult, readonly [TResource, UseReadArgs<TResource> | {}]>,
    'queryKey' | 'queryFn' | 'enabled'
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
    enabled: isEnabled,
    queryFn: async ({ signal }) => {
      const fn = (apiClient.api as any)[resource] as AnyFn;
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

export function useMutate<TResource extends keyof ApiFnMap>(
  resource: TResource,
  options?: Omit<
    UseMutationOptions<Awaited<ReturnType<ApiFn<TResource>>>, Error, MutateVariables<TResource>>,
    'mutationFn'
  >,
) {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const fn = apiClient.api[resource] as ApiFn<TResource>;

  const mutation = useMutation<Awaited<ReturnType<typeof fn>>, Error, MutateVariables<TResource>>({
    ...options,
    mutationFn: ((variables: any) => {
      let args: any[];

      if (resDef.params.length === 1) {
        args = [variables];
      } else if (resDef.params.length === 2) {
        if (variables && typeof variables === 'object' && 'data' in variables) {
          args = [variables.data, variables.params ?? {}];
        } else {
          args = [variables, {}];
        }
      } else {
        args = resDef.params.map((param) => variables[param]);
      }
      return fn(...args);
    }) as MutationFunction<Awaited<ReturnType<typeof fn>>, MutateVariables<TResource>>,
  });

  const validationErrors = useGetValidationErrors(mutation.error);

  return {
    ...mutation,
    validationErrors,
  } as UseMutationResult<Awaited<ReturnType<typeof fn>>, Error, MutateVariables<TResource>, unknown> & {
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

export interface PromptHotkeysConfig {
  /** Function to call when Enter is pressed (confirm action) */
  onConfirm?: () => void;
  /** Function to call when Escape is pressed (cancel/close action) */
  onCancel?: () => void;
  /** Whether the hotkeys are enabled. Defaults to true */
  enabled?: boolean;
  /** Whether to ignore hotkeys when inside input/textarea elements. Defaults to true */
  ignoreInputs?: boolean;
  /** Whether the confirm action is disabled (e.g., form validation failed) */
  confirmDisabled?: boolean;
}

/**
 * Hook that provides standard prompt/dialog hotkey behavior:
 * - Enter: Confirm/submit action
 * - Escape: Cancel/close action
 */
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
