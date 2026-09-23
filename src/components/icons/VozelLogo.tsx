import VozelMark from "./VozelMark";

const VozelLogo = ({
  width = 120,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => (
  <div
    className={`inline-flex items-center justify-center gap-[0.35em] text-logo-primary font-bold tracking-tight ${className ?? ""}`}
    style={{ width, fontSize: width / 5.8 }}
    aria-label="Vozel"
  >
    <VozelMark width="1.5em" height="1.5em" />
    {/* Product names are intentionally not translated. */}
    {/* eslint-disable-next-line i18next/no-literal-string */}
    <span>vozel</span>
  </div>
);

export default VozelLogo;
