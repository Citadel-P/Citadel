import { PullImageReply, PullImageRequest } from '@/api/_generated';
import { SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Check, CircleX, LoaderCircle } from 'lucide-react';
import { usePOSTPullImageStream } from './hooks/usePOSTPullImageStream';
import { useEffect, useRef, useState } from 'react';
import { useContextSelector } from 'use-context-selector';
import { AppContext } from '@/AppProvider';
import { ImagesContext } from './ImagesProvider';
import { Highlight, themes } from 'prism-react-renderer';
import { toast } from 'sonner';
export interface PullProgressSheetProps {
  imageTag: string;
  repository: string;
}
export default function PullProgressSheetContent({ sheetProps }: { sheetProps: PullProgressSheetProps }) {
  const [streamData, setStreamData] = useState<string[] | undefined>(undefined);
  const handleChunkReceived = (chunk: string) => {
    setStreamData((prevChunks) => [...(prevChunks ?? []), chunk]);
  };
  const { isPending, isSuccess, error, mutate } = usePOSTPullImageStream(handleChunkReceived);
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform)!;
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);
  const [pullError, setPullError] = useState<string | undefined>();
  const scrollRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    const controller = new AbortController();
    const request: PullImageRequest = {
      platformId: currentPlatform?.id ?? '',
      registryName: selectedRegistry?.name ?? '',
      repositoryName: sheetProps.repository ?? '',
      imageTag: sheetProps.imageTag ?? '',
    };
    mutate({ ...request, signal: controller.signal });

    return () => {
      controller.abort();
    };
  }, [currentPlatform, selectedRegistry, sheetProps, mutate]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [streamData]);

  useEffect(() => {
    if (isSuccess) {
      const data = streamData?.join('\n');
      if (data) {
        const response = JSON.parse(data) as PullImageReply[];
        const errors = response.filter((s) => s.errorMessage).map((s) => s.errorMessage);
        if (errors && errors.length > 0) {
          const error = errors?.at(0) ?? undefined;
          setPullError(error);
          toast.error('Error', {
            description: error,
          });
        } else {
          toast.success('Image pulled successfully');
        }
      }
    }
  }, [isSuccess, streamData]);
  return (
    <SheetContent side="bottom">
      <SheetHeader>
        <SheetTitle className="flex items-center gap-1">
          <div>Pulling {sheetProps.imageTag}</div>
          <div>
            {isPending ? (
              <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />
            ) : pullError || error ? (
              <CircleX className="text-red-500 ml-1 h-5 w-5 " />
            ) : (
              <Check className="text-green-500 ml-1 h-5 w-5 " />
            )}
          </div>
        </SheetTitle>
        <SheetDescription></SheetDescription>
      </SheetHeader>
      <Highlight
        theme={themes.nightOwl}
        code={isPending && streamData?.length === 0 ? 'Loading...' : (streamData?.join('\n') ?? '')}
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
