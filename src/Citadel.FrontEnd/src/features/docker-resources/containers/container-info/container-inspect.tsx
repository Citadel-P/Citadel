import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';

const ContainerInspect = ({ containerId }: { containerId: string | undefined }) => {
  const { data, isSuccess, isLoading } = useRead('inspectContainer', { id: containerId });

  const code = useMemo(() => {
    if (isLoading) return '// Loading container inspection data...';

    if (isSuccess && data?.data) {
      return JSON.stringify(data.data, null, 2);
    }

    return '// No data available';
  }, [isLoading, isSuccess, data]);

  return (
    <div className="flex-1 w-full border rounded-md overflow-hidden bg-slate-50 dark:bg-zinc-950">
      <MonacoEditor
        value={code}
        language="json"
        filename={`inspect-${containerId?.slice(0, 8)}.json`}
        className="my-0 mx-0 min-h-[600px]"
        readOnly={true}
        folding={true}
      />
    </div>
  );
};

export default ContainerInspect;
