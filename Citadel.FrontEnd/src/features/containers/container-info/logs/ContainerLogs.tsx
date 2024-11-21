import { useContainerLogsContext } from './ContainerLogsProvider';
import { CodeBlock } from 'react-code-block';
import { useEffect, useState } from 'react';

const ContainerLogs = () => {
  const { logs, isPending } = useContainerLogsContext();
  const [logsBuffer, setLogsBuffer] = useState(logs);

  useEffect(() => {
    setLogsBuffer([...logsBuffer, ...logs]);
  }, [logs]);

  return (
    <CodeBlock code={logsBuffer.join('\n')} language="js">
      <CodeBlock.Code className="bg-card-foreground dark:bg-card !p-6 rounded-sm shadow-sm w-full overflow-auto max-w-[1400px] max-h-[600px] scrollbar-thumb-rounded scrollbar-track-rounded scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
        <div className="table-row">
          <CodeBlock.LineNumber className="table-cell pr-4 text-xs text-gray-500 text-right select-none" />
          <CodeBlock.LineContent className="table-cell text-sm">
            {(isPending || !logsBuffer.length) && <p>loading...</p>}
            <CodeBlock.Token />
          </CodeBlock.LineContent>
        </div>
      </CodeBlock.Code>
    </CodeBlock>
  );
};

export default ContainerLogs;
