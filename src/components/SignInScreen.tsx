import { useEffect, useRef, useState } from "react";
import { Copy, Loader } from "lucide-react";
import type { DeviceCodeStart, DevicePoll } from "../types";
import { FolioBrand } from "./FolioBrand";

interface SignInScreenProps {
  onStart: () => Promise<DeviceCodeStart | null>;
  onPoll: (deviceCode: string) => Promise<DevicePoll | null>;
  onAuthorized: () => void;
}

// Su components/sign-in.tsx: pantalla centrada, columna de 340px,
// "Sign in to <producto>" y el botón primario con el logo de GitHub.
// La parte de email/OTP es de su server y no se porta.
export function SignInScreen({
  onStart,
  onPoll,
  onAuthorized,
}: SignInScreenProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [device, setDevice] = useState<DeviceCodeStart | null>(null);
  const [waiting, setWaiting] = useState(true);
  const finished = useRef(false);

  // Entra y arranca el Device Flow: sin esto la pantalla mostraba otro
  // botón idéntico al del home y el doble-click sobre él movía la
  // ventana. Guard por StrictMode (doble montaje en dev).
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void handleGithubSignIn();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleGithubSignIn() {
    setBusy(true);
    setError("");
    finished.current = false;
    const start = await onStart();
    setBusy(false);
    if (start) {
      setDevice(start);
      setWaiting(true);
    }
  }

  // Polling del Device Flow mientras hay un device_code activo.
  useEffect(() => {
    if (!device || finished.current) return;
    let cancelled = false;
    const tick = async (delayMs: number) => {
      window.setTimeout(async () => {
        if (cancelled || finished.current) return;
        const poll = await onPoll(device.device_code);
        if (cancelled || !poll) return;
        if (poll.status === "authorized") {
          finished.current = true;
          onAuthorized();
        } else if (poll.status === "pending") {
          void tick(device.interval * 1000);
        } else if (poll.status === "slow_down") {
          void tick((device.interval + 5) * 1000);
        } else if (poll.status === "expired") {
          setWaiting(false);
          setError("The code expired. Start the sign-in again.");
        } else if (poll.status === "denied") {
          setWaiting(false);
          setError("Sign-in was denied.");
        }
      }, delayMs);
    };
    void tick((device.interval || 5) * 1000);
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device]);

  return (
    <div
      data-tauri-drag-region="false"
      className="flex min-h-screen items-center justify-center p-4 md:p-6"
    >
      {/* Card elevada: panel es el oscuro-un-punto-más-claro del canvas. */}
      <div className="w-full max-w-[340px] space-y-6 rounded-2xl border border-line bg-panel p-8 shadow-2xl">
        <FolioBrand />
        <p className="-mt-2 text-center text-sm text-ink-dim">
          Sign in with GitHub to open your repositories.
        </p>

        {!device ? (
          <button
            type="button"
            onClick={() => void handleGithubSignIn()}
            disabled={busy}
            className="flex w-full items-center justify-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
          >
            <svg
              role="img"
              viewBox="0 0 24 24"
              xmlns="http://www.w3.org/2000/svg"
              fill="currentColor"
              className="size-4"
            >
              <title>GitHub</title>
              <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
            </svg>
            Sign in with GitHub
            {busy && <Loader className="size-4 animate-spin" />}
          </button>
          ) : (
          <div className="space-y-4 text-center">
            <p className="text-sm text-ink-dim">
              Enter this code at{" "}
              <span className="font-medium text-ink">
                github.com/login/device
              </span>
            </p>
            <div className="flex items-center justify-center gap-2">
              <code className="rounded-md border border-line bg-raised px-4 py-2 font-mono text-xl tracking-widest">
                {device.user_code}
              </code>
              <button
                onClick={() =>
                  void navigator.clipboard.writeText(device.user_code)
                }
                title="Copy code"
                className="rounded-md border border-line p-2 hover:bg-raised"
              >
                <Copy size={14} />
              </button>
            </div>
            <a
              href={device.verification_uri}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
            >
              Open GitHub
            </a>
            <p className="text-xs text-ink-dim">
              {waiting ? "Waiting for authorization…" : ""}
            </p>
          </div>
          )}

        {error && <p className="text-sm text-danger">{error}</p>}
      </div>
    </div>
  );
}
