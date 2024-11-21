import { PlatformView } from '@/api/_generated';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';
import { useGETPlatforms } from './hooks/useGETPlatforms';
import usePlatformHub from './hooks/usePlatformHub';

interface IContext {
  isLoading: boolean;
  platforms: PlatformView[] | undefined;
}
interface IProps {
  children?: React.ReactNode;
}

const PlatformsContext = createContext<IContext | undefined>(undefined);

const PlatformsProvider: React.FC<IProps> = ({ children }) => {
  const { data, isLoading, isSuccess } = useGETPlatforms();
  const { platformMessage } = usePlatformHub();
  let platforms: PlatformView[] | undefined;

  if (isSuccess && data?.data) {
    platforms = data.data.platforms!;
  }

  if (platformMessage) {
    platforms = platforms?.map((s) => (s.id === platformMessage.id ? platformMessage : s));
  }

  return (
    <PlatformsContext.Provider
      value={{
        isLoading,
        platforms,
      }}>
      {children}
    </PlatformsContext.Provider>
  );
};

export default PlatformsProvider;

export const usePlatformsContext = () => useRequiredContext(PlatformsContext);
