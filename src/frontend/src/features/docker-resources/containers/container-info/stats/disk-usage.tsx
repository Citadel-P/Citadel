import { IoUsage, type IoUsageProps } from '@/components/custom/stats-io-usage';

export default function DiskUsage(props: IoUsageProps) {
  return <IoUsage {...props} kind="disk" />;
}
