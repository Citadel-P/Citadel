import { PullImageRequest, PullImageResult } from '@/api/generated/api.types';
import { SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Check, CircleX, LoaderCircle } from 'lucide-react';
import { usePOSTPullImageStream } from './hooks/usePOSTPullImageStream';
import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useImagesContext } from './ImagesContext';
import { Highlight, themes } from 'prism-react-renderer';
import { toast } from 'sonner';
import { useAppContext } from '@/AppContext';

export interface PullProgressSheetProps {
  imageTag: string;
  repository: string;
}

export default function PullProgressSheetContent({ sheetProps }: { sheetProps: PullProgressSheetProps }) {
  const [streamData, setStreamData] = useState<string[]>([]);
  const [pullError, setPullError] = useState<string | undefined>();
  const scrollRef = useRef<HTMLPreElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null); // Persist the AbortController

  const { currentPlatform } = useAppContext();
  const { selectedRegistry } = useImagesContext();

  const handleChunkReceived = useCallback((chunk: string) => {
    setStreamData((prevChunks) => [...prevChunks, chunk]);
  }, []);

  const { isPending, isSuccess, error, mutate } = usePOSTPullImageStream(handleChunkReceived);

  // Memoized request object
  const pullRequest = useMemo<PullImageRequest>(
    () => ({
      registryName: selectedRegistry?.name ?? '',
      repositoryName: sheetProps.repository,
      platformId: currentPlatform?.id ?? '',
      imageTag: sheetProps.imageTag,
    }),
    [currentPlatform, selectedRegistry, sheetProps.imageTag, sheetProps.repository],
  );

  // Start the pull image stream
  useEffect(() => {
    // Create a new AbortController only if one doesn't already exist
    if (!abortControllerRef.current) {
      abortControllerRef.current = new AbortController();
    }

    // Start the pull request
    mutate({ ...pullRequest, signal: abortControllerRef.current.signal });

    return () => {
      // Abort the request when the component unmounts
      abortControllerRef.current?.abort();
      abortControllerRef.current = null; // Reset the controller
    };
  }, [mutate, pullRequest]);

  // Auto-scroll to the bottom of the log
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [streamData]);

  // Handle success and errors
  useEffect(() => {
    if (isSuccess) {
      try {
        const data = streamData.join('\n');
        const response = JSON.parse(data) as PullImageResult[];
        const errors = response.filter((s) => s.errorMessage).map((s) => s.errorMessage);

        if (errors.length > 0) {
          const error = errors[0];
          setPullError(error ?? 'Failed to pull');
          toast.error('Error', { description: error });
        } else {
          toast.success('Image pulled successfully');
        }
      } catch (r) {
        setPullError('Failed to parse response');
        toast.error('Error', { description: 'Failed to parse response' + r });
      }
    }
  }, [isSuccess, streamData]);

  // Determine the status icon
  const statusIcon = useMemo(() => {
    if (isPending) return <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />;
    if (pullError || error) return <CircleX className="text-red-500 ml-1 h-5 w-5" />;
    return <Check className="text-green-500 ml-1 h-5 w-5" />;
  }, [isPending, pullError, error]);

  return (
    <SheetContent side="bottom">
      <SheetHeader>
        <SheetTitle className="flex items-center gap-1">
          <div>Pulling {sheetProps.imageTag}</div>
          <div>{statusIcon}</div>
        </SheetTitle>
        <SheetDescription></SheetDescription>
      </SheetHeader>
      <Highlight
        theme={themes.nightOwl}
        code={isPending && streamData.length === 0 ? 'Loading...' : streamData.join('\n')}
        language="tsx">
        {({ style, tokens, getLineProps, getTokenProps }) => (
          <pre
            ref={scrollRef}
            style={style}
            className="bg-card-foreground dark:bg-card p-6! rounded-none shadow-xs w-full overflow-auto max-w-full max-h-[300px] scrollbar-thumb-rounded scrollbar-track-rounded scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
            {tokens.map((line, i) => (
              <div key={i} {...getLineProps({ line })} className="table-row flex-col-reverse">
                <span className="table-cell pr-4 text-xs text-gray-500 text-right select-none">{i + 1}</span>
                {line.map((token, key) => (
                  <span key={key} {...getTokenProps({ token })} className="text-sm" />
                ))}
              </div>
            ))}
          </pre>
        )}
      </Highlight>
    </SheetContent>
  );
}
