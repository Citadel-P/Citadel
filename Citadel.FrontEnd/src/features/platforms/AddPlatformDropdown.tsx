import { ChevronDown } from 'lucide-react';
import { useNavigate } from 'react-router';
import useAnimation from '@/hooks/useAnimation';

const platforms = [
  {
    title: 'Docker Standalone',
    link: 'add-docker-platform',
  },
  {
    title: 'Docker Swarm',
    link: 'add-swarm-platform',
  },
];

export const AddPlatformDropdown = () => {
  const navigate = useNavigate();
  const { ref, open, setOpen } = useAnimation('dropDown');

  return (
    <div className="relative ml-3" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className="inline-flex items-center text-primary-foreground bg-primary hover:bg-primary/80 font-medium rounded-sm text-xs px-2.5 py-2.5"
        type="button">
        <span className="sr-only">Open add menu</span>
        Add Platform
        <ChevronDown className="ml-1 h-4 w-4 text-primary-foreground" />
      </button>
      {open && (
        <div className="absolute right-0 z-20 mt-2 w-52 drop-shadow-md origin-top-right transform rounded-md bg-background py-0 shadow-custom ring-1 ring-transparent ring-opacity-5 transition focus:outline-none">
          <ul className="my-2 mx-2 flex flex-col">
            {platforms.map((item, index) => (
              <li key={index} className="flex">
                <div
                  className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card"
                  role="presentation"
                  onClick={() => navigate(item.link)}>
                  {item.title}
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
