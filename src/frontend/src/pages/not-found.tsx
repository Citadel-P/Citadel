import { PageError } from '@/components/custom/page-error';

export default function NotFound() {
  return <PageError status={404} />;
}
