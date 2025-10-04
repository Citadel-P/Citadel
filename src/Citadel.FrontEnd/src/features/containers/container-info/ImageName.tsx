import { ImageView } from '@/api/_generated';
import { truncate } from '@/lib/truncate';
import { Link } from 'react-router';

export const ImageName = ({ image }: { image: ImageView | undefined }) => {
  if (image === undefined || image?.name === undefined) return <div className="text-muted">{'<none>'}</div>;
  return (
    <Link to={`/platforms/${image.platformId}/images/${image?.imageId?.slice(0, 24)}`} className="table-link">
      {truncate(image?.name, 24)}
    </Link>
  );
};
