export interface IProps {
  onClick: () => void;
}

export const PorfileButton = ({ onClick }: IProps) => {
  return (
    <button onClick={onClick} className="relative flex rounded-full bg-card text-sm" type="button">
      <span className="sr-only">Open user menu</span>
      <img className="h-9 w-9 rounded-md" src="https://avatars.githubusercontent.com/u/993610?v=4" alt="" />
    </button>
  );
};
