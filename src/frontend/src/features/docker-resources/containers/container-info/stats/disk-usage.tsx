import { IoUsage, type IoUsageProps } from './io-usage';

export default function DiskUsage(props: IoUsageProps) {
  return <IoUsage {...props} kind="disk" />;
}
