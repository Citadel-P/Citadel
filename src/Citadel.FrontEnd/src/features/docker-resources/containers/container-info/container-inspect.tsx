import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { KnownResourceName } from '@/api/types';

type InspectProps = {
  readKey: KnownResourceName;
  id?: string | undefined;
  filename?: string | undefined;
  loadingMessage?: string;
  noDataMessage?: string;
};

function Inspect({
  readKey,
  id,
  filename,
  loadingMessage = '// Loading container inspection data...',
  noDataMessage = '// No data available',
}: InspectProps) {
  const { data, isSuccess, isLoading } = useRead(readKey, { id });

  const code = useMemo(() => {
    if (isLoading) return loadingMessage;

    if (isSuccess && data?.data) {
      return JSON.stringify(data.data, null, 2);
    }

    return noDataMessage;
  }, [isLoading, isSuccess, data, loadingMessage, noDataMessage]);
  
  return (
    <div className="flex-1 w-full border rounded-md overflow-hidden bg-slate-50 dark:bg-zinc-950">
      <MonacoEditor
        value={code}
        language="json"
        filename={filename}
        className="my-0 mx-0 min-h-150"
        readOnly={true}
        folding={true}
      />
    </div>
  );
}

export const ContainerInspect = ({ containerId }: { containerId: string | undefined }) => {
  return <Inspect readKey="inspectContainer" id={containerId} filename={`inspect-${containerId}.json`} />;
};

export const DeploymentInspect = ({ deploymentId }: { deploymentId: string | undefined }) => {
  return <Inspect readKey="inspectDeployment" id={deploymentId} filename={`inspect-${deploymentId}.json`} />;
};
