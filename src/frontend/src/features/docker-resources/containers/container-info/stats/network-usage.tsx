import { IoUsage, type IoUsageProps } from '@/components/custom/stats-io-usage';

export default function NetworkUsage(props: IoUsageProps) {
  return <IoUsage {...props} kind="network" />;
}
