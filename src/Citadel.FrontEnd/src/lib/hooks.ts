import { resources } from '@/api/generated/resources';
import {
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
  PluralResourceMap,
  ResourceResponse,
  ResourceType,
  ReversePluralResourceMap,
  UseMutateVariables,
  UseReadArgs,
} from '@/api/types';
import { useGetValidationErrors, getValidationErrors } from '@/hooks/useGetValidationErrors';
import { toast } from 'sonner';
import { useParams, useNavigate } from 'react-router';
import {
  ApplyDeploymentInput,
  ApplyStackInput,
  QueueBackupRunInput,
  RunAutomationActionInput,
  TestAutomationActionInput,
  ProblemDetails,
  PullImageInput,
  RollbackStackInput,
} from '@/api/generated/api.types';
import { useAuthContext } from '@/features/auth/auth-context';

const EMPTY_ARGS = Object.freeze({});
type EmptyArgs = Record<string, never>;

export function useRead<
  TResource extends KnownResourceName,
  TResult extends ResourceResponse<TResource> = ResourceResponse<TResource>,
>(
  resource: TResource,
  args?: UseReadArgs<TResource>,
  options?: Omit<
    UseQueryOptions<TResult, Error, TResult, readonly [TResource, UseReadArgs<TResource> | EmptyArgs]>,
    'queryKey' | 'queryFn'
  >,
): UseQueryResult<TResult, Error> {
  const { apiClient } = useApiClientContext();
  const resDef = resources[resource];
  if (!resDef) throw new Error(`Unknown resource: ${String(resource)}`);

  const stableArgs = args ?? EMPTY_ARGS;
  const queryKey = useMemo(() => [resource, stableArgs] as const, [resource, stableArgs]);

  const isEnabled =
    !!apiClient &&
    resDef.requiredParams.every((p) => {
      const topLevelValue = (args as any)?.[p];
      if (topLevelValue != null) return true;

      const queryValue = (args as any)?.query?.[p];
      return queryValue != null;
    });

  return useQuery<TResult, Error, TResult, readonly [TResource, UseReadArgs<TResource> | EmptyArgs]>({
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
    mutationFn: (variables: TVariables) => {
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

      return fn(...args) as Promise<ResourceResponse<TResource>>;
    },
  });

  const validationErrors = useGetValidationErrors(mutation.error);

  return {
    ...mutation,
    validationErrors,
  } as UseMutationResult<ResourceResponse<TResource>, Error, TVariables, unknown> & {
    validationErrors: ReturnType<typeof useGetValidationErrors>;
  };
}

export const useResourceParamType = (): { type: ResourceType; tab?: ResourceType } => {
  const { type, tab } = useParams();

  if (type === 'automation') return { type: 'AutomationAction' };

  const matchPlural = (value?: string) =>
    value ? Object.values(PluralResourceMap).find((plural) => plural.toLowerCase() === value.toLowerCase()) : undefined;

  if (tab) {
    const tabPlural = matchPlural(tab);
    const typePlural = matchPlural(type);
    if (tabPlural && typePlural) {
      return {
        type: ReversePluralResourceMap[typePlural] as ResourceType,
        tab: ReversePluralResourceMap[tabPlural] as ResourceType,
      };
    }
  }

  if (type === 'platforms' || type === undefined) return { type: 'Platform' };
  if (type === 'registries') return { type: 'Registry' };
  if (type === 'activities') return { type: 'Activity' };
  if (type === 'alert-rules') return { type: 'AlertRule' };
  if (type === 'git-repos') return { type: 'GitRepository' };
  if (type === 'oidc-providers') return { type: 'OidcProvider' };
  if (type === 'backup-repositories') return { type: 'BackupRepository' };
  if (type === 'backup-policies') return { type: 'BackupPolicy' };
  if (type === 'build-pools') return { type: 'BuildAgentPool' };
  if (type === 'access') return { type: 'Access' };

  const typePlural = matchPlural(type);
  if (typePlural) {
    return { type: ReversePluralResourceMap[typePlural] as ResourceType };
  }

  return { type: type as ResourceType };
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
        if ((event.query.meta as { suppressErrorToast?: boolean } | undefined)?.suppressErrorToast) return;
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

type PulledStreamProps =
  | PullImageInput
  | ApplyDeploymentInput
  | ApplyStackInput
  | RollbackStackInput
  | RunAutomationActionInput
  | QueueBackupRunInput
  | TestAutomationActionInput;

const usePulledStream = (onChunkReceived: (chunk: string) => void, endpoint: string, onMutate?: () => void) => {
  const { apiClient } = useApiClientContext();
  const { accessToken } = useAuthContext();

  const mutationFn = async (param: PulledStreamProps & Cancellable) => {
    const { signal, ...body } = param;
    const normalizedBaseUrl = (apiClient?.baseUrl ?? '').replace(/\/+$/, '');
    const normalizedEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
    const requestUrl = `${normalizedBaseUrl}${normalizedEndpoint}`;

    const response = await fetch(requestUrl, {
      method: 'POST',
      credentials: 'include',
      signal,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const responseText = await response.text().catch(() => '');
      throw new Error(responseText || `Network response was not ok: ${response.status} ${response.statusText}`);
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

      const finalChunk = decoder.decode();
      if (finalChunk) {
        onChunkReceived(finalChunk);
      }
    } catch (error) {
      console.error('Error while reading stream:', error);
      throw error;
    } finally {
      reader.releaseLock();
    }
  };

  const { mutate, isPending, isSuccess, error } = useMutation({ mutationFn, onMutate });

  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, error, validationErrors };
};

type StreamStatus = 'pending' | 'success' | 'warning' | 'error';
type StreamLogSeverity = 'info' | 'success' | 'warning' | 'error';
type StreamLogEntry = {
  message: string;
  severity?: StreamLogSeverity;
};

interface StreamProgressState {
  lines: string[];
  logs: StreamLogEntry[];
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
  pendingMessage?: string;
  compactDockerComposeOutput?: boolean;
  streamFieldIsMetadata?: boolean;
  // A predicate to check if an item in the stream represents an error
  getError?: (item: TItem) => string | undefined | null;
  getMessageSeverity?: (item: TItem) => StreamLogSeverity | undefined | null;
}

const formatBytes = (bytes: number) => {
  if (!bytes || bytes <= 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

const getProgressBar = (current: number, total: number) => {
  if (!total || total <= 0) return '';
  const size = 15; // Width of the bar
  const progress = Math.min(Math.round((current / total) * size), size);
  return ` [${'='.repeat(Math.max(0, progress - 1))}>${' '.repeat(Math.max(0, size - progress))}]`;
};

type CompactedComposeLine = {
  kind: 'active' | 'history' | 'ignore';
  key?: string;
  deletePrefixes?: string[];
  line?: string;
};

const dockerComposeStatuses = [
  'Pulling fs layer',
  'Download complete',
  'Pull complete',
  'Already exists',
  'Downloading',
  'Extracting',
  'Waiting',
  'Pulling',
  'Pulled',
  'Recreating',
  'Recreated',
  'Creating',
  'Created',
  'Removing',
  'Removed',
  'Starting',
  'Started',
  'Stopping',
  'Stopped',
  'Running',
  'Healthy',
  'Unhealthy',
  'Skipped',
  'Error',
  'Failed',
];

const dockerComposeLinePattern = new RegExp(
  `^\\s*(?:(Network|Volume|Container)\\s+)?(.+?)\\s+(${dockerComposeStatuses.join('|')})(?:\\s+(.*))?\\s*$`,
  'i',
);

const dockerComposeSplitPattern = new RegExp(
  `\\s{2,}(?=(?:(?:Network|Volume|Container)\\s+)?\\S+\\s+(?:${dockerComposeStatuses.join('|')})\\b)`,
  'i',
);

function splitDockerComposeOutput(value: string): string[] {
  return value
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .split('\n')
    .flatMap((line) => line.split(dockerComposeSplitPattern))
    .map((line) => line.trim())
    .filter(Boolean);
}

function compactDockerComposeLine(value: string): CompactedComposeLine | undefined {
  const summary = value.match(/^\[\+\]\s*(.+)$/);
  if (summary) {
    return { kind: 'active', key: 'compose:summary', line: `Docker Compose: ${summary[1].trim()}` };
  }

  const match = value.match(dockerComposeLinePattern);
  if (!match) return undefined;

  const [, resourceType, name, rawStatus, details] = match;
  const status = rawStatus.toLowerCase();
  const keyName = name.trim();

  if (isDockerLayerId(keyName)) {
    const key = `compose:layer:${keyName}`;
    if (status === 'download complete' || status === 'pull complete' || status === 'already exists') {
      return { kind: 'ignore', key };
    }

    if (status === 'pulling fs layer' || status === 'waiting' || status === 'downloading' || status === 'extracting') {
      return { kind: 'active', key, line: formatDockerLayerStatus(keyName, rawStatus, details) };
    }

    return undefined;
  }

  if (!resourceType && (status === 'pulling' || status === 'pulled')) {
    const key = `compose:image:${keyName}`;
    if (status === 'pulling') {
      return { kind: 'active', key, line: `Pulling image ${keyName}...` };
    }

    return { kind: 'history', key, deletePrefixes: ['compose:layer:'], line: `Pulled image ${keyName}.` };
  }

  if (!resourceType) return undefined;

  const key = `compose:${resourceType.toLowerCase()}:${keyName}`;
  const resource = `${resourceType.toLowerCase()} ${keyName}`;

  switch (status) {
    case 'creating':
    case 'removing':
    case 'starting':
    case 'stopping':
    case 'recreating':
      return { kind: 'active', key, line: `${toPresentParticiple(rawStatus)} ${resource}...` };
    case 'created':
    case 'removed':
    case 'started':
    case 'stopped':
    case 'recreated':
      return { kind: 'history', key, line: `${capitalize(rawStatus)} ${resource}.` };
    case 'running':
    case 'healthy':
      return { kind: 'history', key, line: `${capitalize(resource)} is ${status}.` };
    case 'unhealthy':
    case 'error':
    case 'failed':
      return { kind: 'history', key, line: `${capitalize(resource)} ${status}.` };
    default:
      return undefined;
  }
}

function isDockerLayerId(value: string) {
  return /^[a-f0-9]{12,64}$/i.test(value);
}

function formatDockerLayerStatus(id: string, status: string, details?: string) {
  const shortId = id.slice(0, 12);
  const normalizedStatus = status.toLowerCase();
  const suffix = details?.trim();

  if (normalizedStatus === 'pulling fs layer') return `Preparing layer ${shortId}`;
  if (normalizedStatus === 'waiting') return `Waiting for layer ${shortId}`;
  if (normalizedStatus === 'extracting') return `Extracting layer ${shortId}${suffix ? ` (${suffix})` : ''}`;

  return `Downloading layer ${shortId}${suffix ? ` ${suffix}` : ''}`;
}

function toPresentParticiple(value: string) {
  const lower = value.toLowerCase();
  if (lower.endsWith('ing')) return capitalize(value);
  if (lower.endsWith('e')) return `${capitalize(value.slice(0, -1))}ing`;
  return `${capitalize(value)}ing`;
}

function capitalize(value: string) {
  if (!value) return value;
  return `${value[0].toUpperCase()}${value.slice(1)}`;
}

export function useStreamProgress<TRequest extends PulledStreamProps, TItem>({
  endpoint,
  request,
  successMessage,
  errorMessageDefault,
  pendingMessage,
  compactDockerComposeOutput,
  streamFieldIsMetadata,
  getError,
  getMessageSeverity,
}: UseStreamProgressOptions<TRequest, TItem>): StreamProgressState {
  const [history, setHistory] = useState<StreamLogEntry[]>([]);
  const [activeItems, setActiveItems] = useState<Map<string, string>>(new Map());
  const [internalError, setInternalError] = useState<string | undefined>();

  const bufferRef = useRef('');
  const abortControllerRef = useRef<AbortController | null>(null);
  const [elapsedMs, setElapsedMs] = useState<number>(0);
  const startRef = useRef<number | null>(null);

  const handleChunkReceived = useCallback(
    (chunk: string) => {
      bufferRef.current += chunk;
      let braceCount = 0;
      let startIndex = -1;
      const newHistory: StreamLogEntry[] = [];
      const updatedActive = new Map<string, string>();
      let processedIndex = 0;
      let inString = false;
      let isEscaped = false;
      const activeKeysToDelete = new Set<string>();
      const activePrefixesToDelete = new Set<string>();
      const addText = (value: string | undefined | null, severity?: StreamLogSeverity | null) => {
        if (!value) return false;
        if (!compactDockerComposeOutput) {
          newHistory.push({ message: value.trimEnd(), severity: severity ?? undefined });
          return true;
        }

        const entries = splitDockerComposeOutput(value);
        if (entries.length === 0) return false;

        for (const entry of entries) {
          const compacted = compactDockerComposeLine(entry);
          if (!compacted) {
            newHistory.push({ message: entry, severity: severity ?? undefined });
            continue;
          }

          if (compacted.kind === 'active' && compacted.key && compacted.line) {
            updatedActive.set(compacted.key, compacted.line);
            activeKeysToDelete.delete(compacted.key);
            continue;
          }

          if (compacted.key) {
            activeKeysToDelete.add(compacted.key);
          }

          compacted.deletePrefixes?.forEach((prefix) => activePrefixesToDelete.add(prefix));

          if (compacted.line) {
            newHistory.push({ message: compacted.line, severity: severity ?? undefined });
          }
        }

        return true;
      };

      for (let i = 0; i < bufferRef.current.length; i++) {
        const char = bufferRef.current[i];

        if (startIndex === -1) {
          if (char === '{') {
            startIndex = i;
            braceCount = 1;
          }
          continue;
        }

        if (inString) {
          if (isEscaped) {
            isEscaped = false;
          } else if (char === '\\') {
            isEscaped = true;
          } else if (char === '"') {
            inString = false;
          }
          continue;
        }

        if (char === '"') {
          inString = true;
        } else if (char === '{') {
          braceCount++;
        } else if (char === '}') {
          braceCount--;
          if (braceCount === 0 && startIndex !== -1) {
            const rawObject = bufferRef.current.substring(startIndex, i + 1);
            processedIndex = i + 1;

            try {
              const item = JSON.parse(rawObject) as TItem & {
                id?: string;
                status?: string;
                progress?: {
                  current?: number;
                  total?: number;
                  units?: string;
                };
                progressMessage?: string;
                stream?: string;
                message?: string;
                type?: string;
                severity?: string | null;
              };
              const { id, status, progress, progressMessage, stream, message, type } = item;
              const errorMessage = getError?.(item);
              const messageSeverity = normalizeStreamLogSeverity(item.severity) ?? getMessageSeverity?.(item);

              if (errorMessage) {
                newHistory.push({ message: errorMessage, severity: 'error' });
                setInternalError(errorMessage);
                continue;
              }

              // Handle simple log messages
              if (
                addText(progressMessage, messageSeverity) ||
                addText(message, messageSeverity) ||
                (!streamFieldIsMetadata && addText(stream, messageSeverity))
              ) {
                continue;
              }

              if (compactDockerComposeOutput && type === 'CommandCompleted') {
                activePrefixesToDelete.add('compose:');
                continue;
              }

              // Handle Docker/Progress items
              if (id) {
                const lowerStatus = (status || '').toLowerCase();
                const progressCurrent = progress?.current ?? 0;
                const progressTotal = progress?.total ?? 0;
                const progressUnits = progress?.units ?? '';

                // Define what constitutes an "Active" item vs a "Log" item
                const isProgressing =
                  lowerStatus.includes('downloading') ||
                  lowerStatus.includes('extracting') ||
                  lowerStatus.includes('pushing');
                const isFinished =
                  lowerStatus.includes('complete') ||
                  lowerStatus.includes('pull complete') ||
                  lowerStatus.includes('exists');

                // Build the display line
                let line = `${id}: ${status}`;
                if (progressTotal > 0) {
                  line += `${getProgressBar(progressCurrent, progressTotal)} ${formatBytes(progressCurrent)}/${formatBytes(progressTotal)}`;
                } else if (progressCurrent > 0) {
                  line += ` ${progressCurrent}${progressUnits}`;
                }

                if (isFinished) {
                  // If it's done, move to history and remove from active map
                  newHistory.push({ message: line });
                  activeKeysToDelete.add(id);
                } else if (isProgressing) {
                  updatedActive.set(id, line);
                } else {
                  newHistory.push({ message: line });
                }
              } else if (status) {
                newHistory.push({ message: status, severity: messageSeverity ?? undefined });
              }
            } catch {
              // If parse fails, we just skip this object
            }
            startIndex = -1;
            inString = false;
            isEscaped = false;
          }
        }
      }

      bufferRef.current = bufferRef.current.slice(processedIndex);

      if (newHistory.length > 0) setHistory((prev) => [...prev, ...newHistory]);
      if (updatedActive.size > 0 || activeKeysToDelete.size > 0 || activePrefixesToDelete.size > 0) {
        setActiveItems((prev) => {
          const next = new Map(prev);
          activePrefixesToDelete.forEach((prefix) => {
            Array.from(next.keys())
              .filter((key) => key.startsWith(prefix))
              .forEach((key) => next.delete(key));
          });
          activeKeysToDelete.forEach((key) => next.delete(key));
          updatedActive.forEach((val, key) => next.set(key, val));
          return next;
        });
      }
    },
    [compactDockerComposeOutput, getError, getMessageSeverity, streamFieldIsMetadata],
  );

  const resetTimer = useCallback(() => {
    startRef.current = null;
    setElapsedMs(0);
  }, []);

  const {
    isPending,
    isSuccess,
    error: streamError,
    mutate,
  } = usePulledStream(handleChunkReceived, endpoint, resetTimer);

  const logs = useMemo(() => {
    const activeLogs = Array.from(activeItems.values()).map((message) => ({ message }));
    const combined = [...history, ...activeLogs];
    if (combined.length === 0 && isPending) return [{ message: pendingMessage ?? 'Connecting to registry...' }];
    if (combined.length === 0 && (internalError || streamError)) {
      return [
        {
          message: internalError || (streamError as Error | null)?.message || errorMessageDefault,
          severity: 'error' as const,
        },
      ];
    }
    return combined;
  }, [history, activeItems, isPending, internalError, streamError, errorMessageDefault, pendingMessage]);

  const text = useMemo(() => logs.map((log) => log.message).join('\n'), [logs]);
  const lines = useMemo(() => history.map((log) => log.message), [history]);
  const hasWarning = useMemo(() => logs.some((log) => log.severity === 'warning'), [logs]);

  useEffect(() => {
    const controller = new AbortController();
    abortControllerRef.current = controller;
    startRef.current = null;
    mutate({ ...request, signal: controller.signal });

    return () => {
      controller.abort();
    };
  }, [mutate, request]);

  useEffect(() => {
    let intervalId: number | undefined;

    if (isPending) {
      if (startRef.current == null) startRef.current = performance.now();

      intervalId = window.setInterval(() => {
        setElapsedMs(Math.max(0, performance.now() - (startRef.current ?? 0)));
      }, 100);
    }

    return () => {
      if (intervalId) window.clearInterval(intervalId);
    };
  }, [isPending]);

  useEffect(() => {
    if (isSuccess && !internalError) toast.success(successMessage);
  }, [isSuccess, internalError, successMessage]);

  const status: StreamStatus = useMemo(() => {
    if (isPending) return 'pending';
    if (internalError || streamError) return 'error';
    if (hasWarning) return 'warning';
    return 'success';
  }, [hasWarning, isPending, internalError, streamError]);

  const elapsedLabel = useMemo(() => (Math.floor(elapsedMs / 100) / 10).toFixed(1).replace('.', ','), [elapsedMs]);

  return {
    lines,
    logs,
    text,
    isPending,
    isSuccess,
    status,
    error: internalError || (streamError as any)?.message,
    elapsedMs,
    elapsedLabel,
  };
}

function normalizeStreamLogSeverity(value: string | null | undefined): StreamLogSeverity | undefined {
  const normalized = value?.trim().toLowerCase();
  return normalized === 'info' || normalized === 'success' || normalized === 'warning' || normalized === 'error'
    ? normalized
    : undefined;
}

export function useSaveResource<TInput = any, TResponse = unknown>({
  mode,
  basePath,
  onCreate,
  onUpdate,
  onRefresh,
  entityName = 'Resource',
  extractName,
}: {
  mode: 'add' | 'edit';
  basePath: string;
  onCreate: (payload: TInput) => Promise<TResponse>;
  onUpdate: (payload: TInput) => Promise<unknown>;
  onRefresh?: () => void;
  entityName?: string;
  extractName?: (payload: TInput, response?: TResponse) => string;
}) {
  const [isPending, setIsPending] = useState(false);
  const navigate = useNavigate();

  const save = async (payload: TInput) => {
    setIsPending(true);
    try {
      const getName = (res?: unknown) => {
        if (extractName) return extractName(payload, res as any);
        const data = (res as any)?.data ?? res ?? payload;
        return data?.name ?? data?.type ?? 'Unknown';
      };

      let savedName = getName();

      if (mode === 'edit') {
        await onUpdate(payload);
        onRefresh?.();
      } else {
        const response = await onCreate(payload);
        const resourceId = (response as any)?.id ?? (response as any)?.data?.id;
        savedName = getName(response);

        if (resourceId) {
          navigate(`/${basePath}/edit/${resourceId}`);
        }
      }

      toast.success(`${entityName} "${savedName}" saved successfully`);
    } finally {
      setIsPending(false);
    }
  };

  return { save, isPending };
}

export function useScrollToTop() {
  return useCallback(() => {
    document.getElementById('main-scroll-container')?.scrollTo({ top: 0, behavior: 'smooth' });
  }, []);
}
