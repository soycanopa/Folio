import { PagesMark } from "./PagesMark";

// Marca de Folio: el logo en verde con una sombra paralela suave del
// mismo verde (glow) y el nombre debajo. Compartido por el home sin
// sesión y la pantalla de sign-in.
export function FolioBrand() {
  return (
    <div className="space-y-3">
      <div
        className="flex justify-center text-primary"
        style={{
          filter:
            "drop-shadow(0 6px 16px color-mix(in oklch, var(--color-primary) 45%, transparent))",
        }}
      >
        <PagesMark size={56} />
      </div>
      <h1 className="text-center text-xl font-semibold tracking-tight">
        Folio
      </h1>
    </div>
  );
}
