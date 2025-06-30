import { SheetContent, SheetFooter, SheetHeader, SheetTitle, SheetDescription, Sheet } from '@/components/ui/sheet';
import { useGETInspect } from './hooks/useGETInspect';
import { useEffect, useState } from 'react';
import { Highlight, themes } from 'prism-react-renderer';
import { useNetworksContext } from './NetworksProvider';
import { useAppContext } from '@/AppProvider';

export function NetworkInspectSheet() {
  const { currentNetwork, sheetOpen, setSheetOpen } = useNetworksContext();
  const { currentPlatform } = useAppContext();
  const { data, isSuccess } = useGETInspect(currentPlatform?.id ?? null, currentNetwork?.id ?? null);
  const [inspectData, setInspectData] = useState<string>('');

  useEffect(() => {
    if (isSuccess && data?.data) {
      setInspectData(JSON.stringify(data.data, null, 2));
    }
  }, [data, isSuccess]);

  return (
    <Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
      <SheetContent side="right" className="w-full sm:!w-[500px] md:!w-[700px] !max-w-none flex flex-col">
        <SheetHeader>
          <SheetTitle>Viewing {currentNetwork?.name} details</SheetTitle>
          <SheetDescription></SheetDescription>
        </SheetHeader>
        <div className="sr-only">Detailed information about the selected network.</div>
        <div className="flex-1 min-h-0 min-w-0 flex flex-col">
          <Highlight theme={themes.nightOwl} code={inspectData} language="tsx">
            {({ style, tokens, getLineProps, getTokenProps }) => (
              <pre
                style={style}
                className="flex-1 min-h-0 min-w-0 bg-card-foreground dark:bg-card p-6 rounded-sm shadow-xs w-full h-full overflow-auto scrollbar-thumb-rounded scrollbar-track-rounded scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
                {tokens.map((line, i) => (
                  <div key={i} {...getLineProps({ line })} className="table-row">
                    {/* Line number */}
                    <span className="table-cell pr-4 text-xs text-gray-500 text-right select-none">{i + 1}</span>
                    {/* Line content */}
                    {line.map((token, key) => (
                      <span key={key} {...getTokenProps({ token })} className="text-sm" />
                    ))}
                  </div>
                ))}
              </pre>
            )}
          </Highlight>
        </div>
        <SheetFooter></SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
