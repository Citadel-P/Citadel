import { useContainerLogsContext } from './ContainerLogsContext';
import CodeHighlight from '@/components/custom/code-highlight';

const ContainerLogs = () => {
  const { logs } = useContainerLogsContext();

  return <CodeHighlight code={logs?.length === 0 ? 'Loading...' : logs!.join('\n')} language="tsx" autoScroll />;
};

export default ContainerLogs;
