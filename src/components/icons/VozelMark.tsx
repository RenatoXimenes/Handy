const VozelMark = ({
  width = 24,
  height = 24,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => (
  <svg
    width={width}
    height={height}
    className={className}
    viewBox="0 0 64 64"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <path
      d="M5 32h8l6-16 9 33 9-33 6 16h12"
      stroke="currentColor"
      strokeWidth="5"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <circle cx="58" cy="32" r="3" fill="currentColor" />
  </svg>
);

export default VozelMark;
