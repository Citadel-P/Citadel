import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import { PullImageRequest, PullImageResult } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { useResourceFilter } from '@/lib/atoms';
import { RegistryView } from '@/api/generated/api.types';
import { usePullImageStream } from './usePullImageStream';
import { toast } from 'sonner';

export type PullImageParams = {
  imageTag: string;
  repository: string;
  registryName?: string;
};

export type ImagePullProgressState = {
  lines: string[];
  text: string;
  isPending: boolean;
  isSuccess: boolean;
  status: 'pending' | 'success' | 'error';
  error?: string;
};

export function usePullProgress(params: PullImageParams): ImagePullProgressState {
  const [lines, setLines] = useState<string[]>([]);
  const [pullError, setPullError] = useState<string | undefined>();
  const abortControllerRef = useRef<AbortController | null>(null);

  const { currentPlatform } = useAppContext();
  const [registryFilter] = useResourceFilter<{ item: RegistryView }>('Registry');

  const handleChunkReceived = useCallback((chunk: string) => {
    setLines((prev) => [...prev, chunk]);
  }, []);

  const { isPending, isSuccess, error, mutate } = usePullImageStream(handleChunkReceived);

  const request: PullImageRequest = useMemo(
    () => ({
      registryName: params.registryName ?? registryFilter?.item?.name ?? '',
      repositoryName: params.repository,
      platformId: currentPlatform?.id ?? '',
      imageTag: params.imageTag,
    }),
    [currentPlatform, registryFilter?.item?.name, params.imageTag, params.repository, params.registryName],
  );

  useEffect(() => {
    if (!abortControllerRef.current) {
      abortControllerRef.current = new AbortController();
    }
    mutate({ ...request, signal: abortControllerRef.current.signal });
    return () => {
      abortControllerRef.current?.abort();
      abortControllerRef.current = null;
    };
  }, [mutate, request]);

  useEffect(() => {
    if (isSuccess) {
      try {
        const data = lines.join('\n');
        const response = JSON.parse(data) as PullImageResult[];
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

  return {
    lines,
    text: lines.length === 0 && isPending ? 'Loading...' : lines.join('\n'),
    isPending,
    isSuccess,
    status,
    error: pullError || (error as any)?.message,
  };
}
