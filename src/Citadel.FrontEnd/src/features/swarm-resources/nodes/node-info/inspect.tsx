import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { useRead } from '@/lib/hooks';
import { MonacoEditor } from '@/lib/monaco';
import { serializeData } from '@/lib/utils';
import { SwarmNodeInfoView } from '../hooks/useNodesGroup';

export const NodeInspect = ({ node }: { node: SwarmNodeInfoView }) => {
  const query = useRead('inspectSwarmNode', { platformId: node.platformId, nodeId: node.id });
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Unable to inspect node'} type="error">
        {problem.detail ?? 'Docker could not inspect this node.'}
      </AlertMessage>
    );
  }

  const value = query.isLoading
    ? '// Loading node inspection data...'
    : query.data?.data
      ? serializeData(query.data.data, 'json')
      : '// No node inspection data available';
  return (
    <div className="w-full flex-1 overflow-hidden rounded-md border bg-slate-50 dark:bg-zinc-950">
      <MonacoEditor
        value={value}
        language="json"
        filename={`inspect-node-${node.id}.json`}
        className="mx-0 my-0 min-h-150"
        readOnly
        folding
      />
    </div>
  );
};
