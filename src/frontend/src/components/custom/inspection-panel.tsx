import { ReactNode, useMemo } from 'react';
import { MonacoEditor } from '@/lib/monaco';
import { serializeData } from '@/lib/utils';

type InspectionPanelProps = {
  data: unknown;
  isLoading?: boolean;
  filename?: string;
  loadingMessage?: string;
  noDataMessage?: string;
  minHeight?: number;
  children?: ReactNode;
};

export const InspectionPanel = ({
  data,
  isLoading = false,
  filename,
  loadingMessage = '// Loading inspection data...',
  noDataMessage = '// No data available',
  minHeight,
  children,
}: InspectionPanelProps) => {
  const value = useMemo(() => {
    if (isLoading) return loadingMessage;
    return data != null ? serializeData(data, 'json') : noDataMessage;
  }, [data, isLoading, loadingMessage, noDataMessage]);

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="w-full flex-1 overflow-hidden rounded-md border bg-slate-50 dark:bg-zinc-950">
        <MonacoEditor
          value={value}
          language="json"
          filename={filename}
          className={minHeight == null ? 'mx-0 my-0 min-h-150' : 'mx-0 my-0'}
          minHeight={minHeight}
          readOnly
          folding
        />
      </div>
      {children}
    </div>
  );
};
