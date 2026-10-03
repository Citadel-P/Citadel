import { NetworkView } from '@/api/generated/api.types';
import { DetailFacts } from '@/components/custom/resource-detail';

export const NetworkInfoTable = ({ network }: { network: NetworkView | undefined }) => {
  if (!network) return null;
  return (
    <DetailFacts
      resource={network}
      items={[
        { label: 'Driver', value: network.driver },
        { label: 'Scope', value: network.scope },
        { label: 'Connected containers', value: Object.keys(network.containers ?? {}).length },
        { label: 'Attachable', value: network.attachable ? 'Yes' : 'No' },
        { label: 'Internal network', value: network.internal ? 'Yes' : 'No' },
        { label: 'Ingress', value: network.ingress ? 'Yes' : 'No' },
      ]}
    />
  );
};
