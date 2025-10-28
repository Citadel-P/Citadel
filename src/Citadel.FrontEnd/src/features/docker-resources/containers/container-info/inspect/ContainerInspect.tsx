import { useParams } from 'react-router';
import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import CodeHighlight from '@/components/custom/code-highlight';

const ContainerInspect = () => {
  const { containerId } = useParams();
  const { data, isSuccess, isLoading } = useRead('inspectContainer', { id: containerId });

  // Memoize the code to avoid recalculating it on every render
  const code = useMemo(() => {
    if (isLoading) return 'Loading...';
    if (isSuccess && data?.data) return JSON.stringify(data.data, null, 2);
    return '';
  }, [isLoading, isSuccess, data]);

  // Define styles for the <pre> element
  const preClassName =
    'bg-card-foreground dark:bg-card p-6 rounded-sm shadow-xs w-full overflow-auto max-w-[1400px] max-h-[650px]';

  return <CodeHighlight code={code} language="tsx" className={preClassName} />;
};

export default ContainerInspect;
