import NotFound from '@/pages/not-found';
import { ComponentType } from 'react';
import { useParams } from 'react-router';
import {
  SwarmConfigDetailsPage,
  SwarmNetworkDetailsPage,
  SwarmSecretDetailsPage,
  SwarmServiceDetailsPage,
  SwarmTaskDetailsPage,
} from './inventory-details';
import { SwarmConfigsPage, SwarmNetworksPage, SwarmSecretsPage, SwarmServicesPage, SwarmTasksPage } from './inventory';
import { SwarmNodeDetailsPage } from './nodes/node-details';
import { SwarmNodesPage } from './nodes';

type SwarmPage = {
  list: ComponentType;
  details: ComponentType;
};

const pages: Record<string, SwarmPage> = {
  nodes: { list: SwarmNodesPage, details: SwarmNodeDetailsPage },
  services: { list: SwarmServicesPage, details: SwarmServiceDetailsPage },
  tasks: { list: SwarmTasksPage, details: SwarmTaskDetailsPage },
  networks: { list: SwarmNetworksPage, details: SwarmNetworkDetailsPage },
  secrets: { list: SwarmSecretsPage, details: SwarmSecretDetailsPage },
  configs: { list: SwarmConfigsPage, details: SwarmConfigDetailsPage },
};

export default function SwarmResourcePage() {
  const { resourceType = '', resourceId } = useParams<{ resourceType: string; resourceId?: string }>();
  const page = pages[resourceType];
  if (!page) return <NotFound />;

  const Page = resourceId ? page.details : page.list;
  return <Page />;
}
