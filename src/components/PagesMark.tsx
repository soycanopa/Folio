// El logo de Pages CMS (su app/icon.svg, MIT): rectángulo redondeado
// con la forma de página doblada. Los colores vienen del contexto
// (currentColor): sobre bg-primary/text-primary-foreground equivale a
// su variante oscura de la plataforma.
export function PagesMark({ size = 16 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 480 480"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M60 132C60 92.2355 92.2355 60 132 60H240.177C259.272 60 277.586 67.5857 291.088 81.0883L398.912 188.912C412.414 202.414 420 220.728 420 239.823V348C420 387.764 387.764 420 348 420H132C92.2355 420 60 387.764 60 348V132Z"
        fill="currentColor"
      />
    </svg>
  );
}
