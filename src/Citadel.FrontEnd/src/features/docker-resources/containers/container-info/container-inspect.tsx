import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import CodeHighlight from '@/components/custom/code-highlight';
import { DockerContainerView } from '@/api/types';

const ContainerInspect = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { data, isSuccess, isLoading } = useRead('inspectContainer', { id: resource?.id });

  const code = useMemo(() => {
    if (isLoading) return 'Loading...';
    if (isSuccess && data?.data) return JSON.stringify(data.data, null, 2);
    return '';
  }, [isLoading, isSuccess, data]);

  return <CodeHighlight code={code} language="tsx" showLineNumbers={true} />;
};

export default ContainerInspect;
