import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { KnownResourceName } from '@/api/types';
import { serializeData } from '@/lib/utils';

type InspectProps = {
  readKey: KnownResourceName;
  id?: string | undefined;
  stackId?: string | undefined;
  containerId?: string | undefined;
  filename?: string | undefined;
  loadingMessage?: string;
  noDataMessage?: string;
};

function Inspect({
  readKey,
  id,
  stackId,
  containerId,
  filename,
  loadingMessage = '// Loading container inspection data...',
  noDataMessage = '// No data available',
}: InspectProps) {
  const readArgs = stackId && containerId ? { stackId, containerId } : { id };
  const { data, isSuccess, isLoading } = useRead(readKey, readArgs as any);

  const code = useMemo(() => {
    if (isSuccess && data?.data != null) {
      return serializeData(data.data, 'json');
    }

    if (isLoading) return loadingMessage;

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

export const StackInspect = ({
  stackId,
  containerId,
}: {
  stackId: string | undefined;
  containerId: string | undefined;
}) => {
  return (
    <Inspect
      readKey="inspectStackContainer"
      stackId={stackId}
      containerId={containerId}
      filename={`inspect-${stackId}-${containerId}.json`}
    />
  );
};
