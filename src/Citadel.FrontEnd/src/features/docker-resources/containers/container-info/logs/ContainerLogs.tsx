import { useContainerLogsContext } from './ContainerLogsContext';
import CodeHighlight from '@/components/custom/code-highlight';

const ContainerLogs = () => {
  const { logs } = useContainerLogsContext();

  return (
    <CodeHighlight
      code={logs?.length === 0 ? 'Loading...' : logs!.join('\n')}
      language="tsx"
      autoScroll
      className="bg-card-foreground dark:bg-card p-6! rounded-sm shadow-xs w-full overflow-auto max-w-[1400px] max-h-[600px]"
    />
  );
};

export default ContainerLogs;
