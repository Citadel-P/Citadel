import { createContext } from 'use-context-selector';
import { useGETRegistries } from '../registries/hooks/useGETRegistries';
import { useEffect, useState } from 'react';
import { RegistryDiscriminator, RegistryView } from '@/api/_generated';
import { useGETExternalImages } from './hooks/useGETExternalImages';
import { useQueryClient } from '@tanstack/react-query';

interface IContext {
  isLoading: boolean;
  isPlatformOnline: boolean;
  registries: RegistryView[];
  setSelectionChange: (name: string) => void;
  selectedRegistry: RegistryView | undefined;
}
interface IProps {
  children?: React.ReactNode;
}

export const ImagesContext = createContext<IContext | undefined>(undefined);

const ImagesProvider: React.FC<IProps> = ({ children }) => {
  const client = useQueryClient();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const [selectedRegistry, setSelectedRegistry] = useState<RegistryView | undefined>();
  
  const isPlatformOnline = true;

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
      setSelectedRegistry(data?.data.registries?.at(0));
    }
  }, [isSuccess, data]);

  function setSelectionChange(name: string) {
    const registry = registries.find((s) => s.name === name);
    if (registry) {
      setSelectedRegistry(registry);
      client.invalidateQueries({ queryKey: ['externalImages', registry.name] });
    }
  }

  return (
    <ImagesContext.Provider
      value={{
        isLoading,
        registries,
        isPlatformOnline,
        selectedRegistry,
        setSelectionChange,
      }}>
      {children}
    </ImagesContext.Provider>
  );
};

export default ImagesProvider;
