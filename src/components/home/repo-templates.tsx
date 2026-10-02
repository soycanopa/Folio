/*
 * Copiado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Source: components/repo/repo-templates.tsx
 * Adapted: la server action handleCopyTemplate (creaba el repo con la
 * GitHub App de su server) → prop onCreateTemplate(repository, name),
 * que usa la API generate de GitHub con el token del usuario. El
 * dropdown de cuentas queda con la única cuenta del token.
 */
import { useRef, useState } from "react";
import templates from "../../lib/templates";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { toast } from "sonner";
import type { SelectAccount } from "./repo-select";

interface RepoTemplatesProps {
  user: { accounts: SelectAccount[] };
  onCreateTemplate: (
    repository: string,
    name: string,
  ) => Promise<string | null>;
}

export function RepoTemplates({ user, onCreateTemplate }: RepoTemplatesProps) {
  const dialogCloseRef = useRef<HTMLButtonElement>(null);

  const [selectedAccount] = useState(
    user?.accounts?.[0],
  );
  const [name, setName] = useState(templates[0].suggested);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [pending, setPending] = useState(false);

  const handleCopy = async (repository: string) => {
    setPending(true);
    try {
      const message = await onCreateTemplate(repository, name);
      if (message !== null) {
        toast.success(message, { duration: 10000 });
        dialogCloseRef.current?.click();
      }
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="flex flex-col gap-y-4">
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-6">
        {templates
          .filter((item) => item.featured === true)
          .map((template) => (
            <div key={template.repository}>
              <button
                onClick={() => {
                  setName(template.suggested);
                  setDialogOpen(true);
                }}
                className="border rounded-md overflow-hidden hover:cursor-pointer hover:bg-accent ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              >
                <img
                  src={template.thumbnail}
                  alt={`Preview for ${template.name}`}
                  className="aspect-video w-full"
                />
                <div className="flex gap-x-2 items-center px-3 py-2 border-t border-t-accent text-sm">
                  <div
                    dangerouslySetInnerHTML={{ __html: template.icon }}
                    className="w-4 h-4 shrink-0"
                  />
                  <div className="font-medium truncate">{template.name}</div>
                </div>
              </button>

              {dialogOpen && (
                <TemplateDialog
                  template={template}
                  account={selectedAccount ?? user.accounts[0]}
                  name={name}
                  pending={pending}
                  dialogCloseRef={dialogCloseRef}
                  onNameChange={setName}
                  onClose={() => setDialogOpen(false)}
                  onCopy={() => handleCopy(template.repository)}
                />
              )}
            </div>
          ))}
      </div>
    </div>
  );
}

const validName = (name: string) => {
  if (!name || name.length > 100) return false;
  const validNameRegex =
    /^(?!\.|\.\.|.*\/|.*\/\.|.*\.\.|.*\/\.)(?!@)(?!.*[~^:?*[\]{}()<>#%&!\\$'"|;,])[^\x20\x7f]*[^\x20\x7f\.]$/;
  return validNameRegex.test(name);
};

function TemplateDialog({
  template,
  account,
  name,
  pending,
  dialogCloseRef,
  onNameChange,
  onClose,
  onCopy,
}: {
  template: (typeof templates)[number];
  account: SelectAccount;
  name: string;
  pending: boolean;
  dialogCloseRef: React.RefObject<HTMLButtonElement | null>;
  onNameChange: (name: string) => void;
  onClose: () => void;
  onCopy: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50" onClick={onClose}>
      <div className="absolute inset-0 bg-black/60" />
      <div
        className="relative mx-auto mt-24 w-full max-w-[425px] rounded-lg border border-line bg-popover p-6 text-popover-foreground shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <form
          className="grid gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            void onCopy();
          }}
        >
          <div className="flex flex-col space-y-1.5 text-center sm:text-left">
            <h2 className="text-lg font-semibold leading-none tracking-tight">
              Copy template
            </h2>
            <p className="text-sm text-muted-foreground">
              This will create a copy of the template repository below under
              the selected account.
            </p>
          </div>
          <a
            href={`https://github.com/${template.repository}`}
            target="_blank"
            className="border rounded-lg transition-all hover:bg-accent focus:bg-accent outline-none flex items-center overflow-hidden relative"
          >
            <img
              src={template.thumbnail}
              alt={`Preview for ${template.name}`}
              className="aspect-video h-20"
            />
            <div className="flex-1 text-left flex flex-col gap-y-1 truncate px-3 py-2 h-full justify-center border-l border-l-accent">
              <div className="tracking-tight truncate font-medium">
                {template.name}
              </div>
              <div className="text-xs text-muted-foreground truncate">
                {template.repository}
              </div>
            </div>
            <div className="absolute top-2 right-2">
              <svg className="h-3 w-3 opacity-50" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M7 7h10v10"/><path d="M7 17 17 7"/></svg>
            </div>
          </a>
          <div className="grid gap-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <span className="text-sm text-right text-muted-foreground">
                Account
              </span>
              <div className="col-span-3 flex items-center gap-2 rounded-md border border-input px-3 py-2 text-sm">
                <img
                  className="h-6 w-6 rounded mr-2"
                  src={`https://github.com/${account.login}.png`}
                  alt={`${account.login}'s avatar`}
                />
                <div className="truncate">{account.login}</div>
              </div>
            </div>
            <div className="grid grid-cols-4 items-start gap-4">
              <label className="h-10 inline-flex items-center justify-end text-sm font-medium">
                Name
              </label>
              <div className="col-span-3">
                <Input
                  required
                  value={name}
                  onChange={(e) => onNameChange(e.target.value)}
                />
              </div>
            </div>
          </div>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <button
              ref={dialogCloseRef}
              type="button"
              onClick={onClose}
              className="hidden"
            />
            <Button type="submit" disabled={!validName(name) || pending}>
              {pending ? "Creating…" : "Create copy"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
