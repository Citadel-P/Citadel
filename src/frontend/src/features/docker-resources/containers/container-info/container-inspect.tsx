import { ReactNode } from 'react';
import { ContainerProcesses } from './container-processes';
import { RuntimeProcesses } from './runtime-processes';
import { useRead } from '@/lib/hooks';
import { InspectionPanel } from '@/components/custom/inspection-panel';
import { KnownResourceName } from '@/api/types';

type InspectProps = {
  readKey: KnownResourceName;
  id?: string | undefined;
  stackId?: string | undefined;
  containerId?: string | undefined;
  filename?: string | undefined;
  loadingMessage?: string;
  noDataMessage?: string;
  children?: ReactNode;
};

function Inspect({
  readKey,
  id,
  stackId,
  containerId,
  filename,
  loadingMessage = '// Loading container inspection data...',
  noDataMessage = '// No data available',
  children,
}: InspectProps) {
  const readArgs = stackId && containerId ? { stackId, containerId } : { id };
  const { data, isSuccess, isLoading } = useRead(readKey, readArgs as any);

  return (
    <InspectionPanel
      data={isSuccess ? data?.data : undefined}
      isLoading={isLoading}
      filename={filename}
      loadingMessage={loadingMessage}
      noDataMessage={noDataMessage}>
      {children}
    </InspectionPanel>
  );
}

export const ContainerInspect = ({
  containerId,
  resourceId,
}: {
  containerId: string | undefined;
  resourceId?: string;
}) => {
  return (
    <Inspect readKey="inspectContainer" id={containerId} filename={`inspect-${containerId}.json`}>
      {resourceId && <ContainerProcesses key={resourceId} resourceId={resourceId} />}
    </Inspect>
  );
};

export const DeploymentInspect = ({
  deploymentId,
  resourceId,
}: {
  deploymentId: string | undefined;
  resourceId?: string | null;
}) => {
  return (
    <Inspect readKey="inspectDeployment" id={deploymentId} filename={`inspect-${deploymentId}.json`}>
      {resourceId && <ContainerProcesses key={resourceId} resourceId={resourceId} />}
    </Inspect>
  );
};

export const StackInspect = ({
  stackId,
  containerId,
  platformId,
  dockerNodeId,
}: {
  stackId: string | undefined;
  containerId: string | undefined;
  platformId: string;
  dockerNodeId?: string | null;
}) => {
  return (
    <Inspect
      readKey="inspectStackContainer"
      stackId={stackId}
      containerId={containerId}
      filename={`inspect-${stackId}-${containerId}.json`}>
      {containerId && (
        <RuntimeProcesses platformId={platformId} containerId={containerId} dockerNodeId={dockerNodeId} />
      )}
    </Inspect>
  );
};
