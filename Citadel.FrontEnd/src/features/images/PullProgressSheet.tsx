import {
  GhcrPackageVersion,
  IImageResponseGitHubPackageResponse,
  PullImageReply,
  PullImageRequest,
} from '@/api/_generated';
import { Badge } from '@/components/ui/badge';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import { ArrowDown, Check, CircleX, LoaderCircle } from 'lucide-react';
import { usePOSTPullImageStream } from './hooks/usePOSTPullImageStream';
import { useEffect, useRef, useState } from 'react';
import { useContextSelector } from 'use-context-selector';
import { AppContext } from '@/AppProvider';
import { ImagesContext } from './ImagesProvider';
import { Highlight, themes } from 'prism-react-renderer';

export default function PullProgressSheet({
  ghPackage,
  version,
}: {
  ghPackage: IImageResponseGitHubPackageResponse;
  version: GhcrPackageVersion;
}) {
  const [streamData, setStreamData] = useState<string[] | undefined>(undefined);
  const handleChunkReceived = (chunk: string) => {
    setStreamData((prevChunks) => [...(prevChunks ?? []), chunk]);
  };
  const { isPending, isSuccess, mutate } = usePOSTPullImageStream(handleChunkReceived);
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const selectedRegistry = useContextSelector(ImagesContext, (v) => v?.selectedRegistry);
  const [pullError, setPullError] = useState<string | undefined>();
  const scrollRef = useRef<HTMLPreElement>(null);

  function onOpenAutoFocus() {
    const request: PullImageRequest = {
      platformId: currentPlatform?.id ?? '',
      registryName: selectedRegistry?.name ?? '',
      packageName: ghPackage.name ?? '',
      imageTag: version.name ?? '',
    };
    mutate(request);
  }

  // Scroll to the bottom whenever streamData changes
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [streamData]);

  function onCloseAutoFocus() {
    setStreamData(undefined);
    setPullError(undefined);
  }

  useEffect(() => {
    if (isSuccess) {
      const data = streamData?.join('\n');
      if (data) {
        const response = JSON.parse(data) as PullImageReply[];
        const errors = response.filter((s) => s.errorMessage).map((s) => s.errorMessage);
        if (errors) {
          setPullError(errors?.at(0) ?? undefined);
        }
      }
    }
  }, [isSuccess, streamData]);
  return (
    <Sheet key={version.id}>
      <SheetTrigger asChild>
        <Badge className="flex text-right cursor-pointer invisible group/versionrowdown group-hover/versionrow:visible truncate rounded-full hover:bg-primary/90">
          <span>Pull</span>
          <ArrowDown className="ml-1 h-3.5 w-3.5 text-background group-hover/versionrowdown:animate-bounce" />
        </Badge>
      </SheetTrigger>
      <SheetContent onOpenAutoFocus={onOpenAutoFocus} onCloseAutoFocus={onCloseAutoFocus} side="bottom">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-1">
            <div>Pulling {version.name}</div>
            <div>
              {isPending ? (
                <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />
              ) : pullError ? (
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
    </Sheet>
  );
}
