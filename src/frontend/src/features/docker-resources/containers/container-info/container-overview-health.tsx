import { useLocation } from 'react-router';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerHealthCard } from './container-health-card';

export function ContainerOverviewHealth({ resource }: { resource: ContainerDetailsView }) {
  const location = useLocation();
  return <ContainerHealthCard resource={resource} initiallyOpen={location.hash === '#health'} />;
}
