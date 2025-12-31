import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import { PullImageInput, PullImageStreamItem } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { useResourceFilter } from '@/lib/atoms';
import { RegistryView } from '@/api/generated/api.types';
import { usePullImageStream } from './usePullImageStream';
import { toast } from 'sonner';

export type PullImageParams = {
  imageTag: string;
  registryId: string;
};

export type ImagePullProgressState = {
  lines: string[];
  text: string;
  isPending: boolean;
  isSuccess: boolean;
  status: 'pending' | 'success' | 'error';
  error?: string;
  elapsedMs: number;
  elapsedLabel: string;
};

export function usePullProgress(params: PullImageParams): ImagePullProgressState {
  const [lines, setLines] = useState<string[]>([]);
  const [pullError, setPullError] = useState<string | undefined>();
  const abortControllerRef = useRef<AbortController | null>(null);
  const [elapsedMs, setElapsedMs] = useState<number>(0);
  const startRef = useRef<number | null>(null);
  const timerRef = useRef<number | null>(null);

  const { currentPlatform } = useAppContext();
  const [registryFilter] = useResourceFilter<{ item: RegistryView }>('Registry');

  const handleChunkReceived = useCallback((chunk: string) => {
    setLines((prev) => [...prev, chunk]);
  }, []);

  const { isPending, isSuccess, error, mutate } = usePullImageStream(handleChunkReceived);

  const request: PullImageInput = useMemo(
    () => ({
      registryId: params.registryId ?? registryFilter?.item?.name ?? '',
      platformId: currentPlatform?.id ?? '',
      imageTag: params.imageTag,
    }),
    [currentPlatform, registryFilter?.item?.name, params.imageTag, params.registryId],
  );

  useEffect(() => {
    if (!abortControllerRef.current) {
      abortControllerRef.current = new AbortController();
    }
    // Start the stream request
    mutate({ ...request, signal: abortControllerRef.current.signal });
    return () => {
      abortControllerRef.current?.abort();
      abortControllerRef.current = null;
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [mutate, request]);

  useEffect(() => {
    if (isSuccess) {
      try {
        const data = lines.join('\n');
        const response = JSON.parse(data) as PullImageStreamItem[];
        const errors = response.filter((s) => s.errorMessage).map((s) => s.errorMessage);
        if (errors.length > 0) {
          const err = errors[0];
          setPullError(err ?? 'Failed to pull');
          toast.error('Error', { description: err });
        } else {
          toast.success('Image pulled successfully');
        }
      } catch (r) {
        const msg = 'Failed to parse response';
        setPullError(msg);
        toast.error('Error', { description: msg + r });
      }
    }
  }, [isSuccess, lines]);

  const status: ImagePullProgressState['status'] = useMemo(() => {
    if (isPending) return 'pending';
    if (pullError || error) return 'error';
    return 'success';
  }, [isPending, pullError, error]);

  // Manage timer lifecycle based on pending status
  useEffect(() => {
    if (isPending) {
      if (startRef.current == null) startRef.current = performance.now();
      if (timerRef.current == null) {
        timerRef.current = window.setInterval(() => {
          if (startRef.current != null) setElapsedMs(Math.max(0, performance.now() - startRef.current));
        }, 100);
      }
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
    return () => {
      // cleanup when unmounting or deps change
      if (!isPending && timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [isPending]);

  const elapsedLabel = useMemo(() => {
    const tenths = Math.floor(elapsedMs / 100) / 10; // e.g., 59.5
    return tenths.toFixed(1).replace('.', ',');
  }, [elapsedMs]);

  return {
    lines,
    text: lines.length === 0 && isPending ? 'Loading...' : lines.join('\n'),
    isPending,
    isSuccess,
    status,
    error: pullError || (error as any)?.message,
    elapsedMs,
    elapsedLabel,
  };
}
