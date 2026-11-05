import { IoUsage, type IoUsageProps } from './io-usage';

export default function NetworkUsage(props: IoUsageProps) {
  return <IoUsage {...props} kind="network" />;
}
