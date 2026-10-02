/*
 * Copiado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Source: components/repo/repo-select.tsx
 * Adapted: useUser() → props (user con una cuenta, la del token);
 * fetch("/api/repos/…") → prop loadRepos(keyword) que filtra como el
 * endpoint de ellos; next/link → button que abre el proyecto.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { useDebounce } from "use-debounce";
import { formatDistanceToNow } from "date-fns";
import { Button, buttonVariants } from "../ui/button";
import { ButtonGroup } from "../ui/button-group";
import { Input } from "../ui/input";
import { Skeleton } from "../ui/skeleton";
import { Settings } from "lucide-react";
import { cn } from "../../lib/utils";

export interface SelectableRepo {
  repo: string;
  owner: string;
  private: boolean;
  updatedAt: string;
  defaultBranch: string;
}

export interface SelectAccount {
  login: string;
  repositorySelection: "all" | "selected";
}

interface RepoSelectProps {
  user: { accounts: SelectAccount[] };
  loadRepos: (keyword: string) => Promise<SelectableRepo[]>;
  onOpenRepo: (result: SelectableRepo) => void;
  onAccountSelect?: (account: SelectAccount) => void;
  onSignOut?: () => void;
}

export function RepoSelect({ user, loadRepos, onOpenRepo, onAccountSelect, onSignOut }: RepoSelectProps) {
  const accounts = useMemo(() => {
    return user.accounts || [];
  }, [user]);

  const [selectedAccount, setSelectedAccount] = useState(accounts[0]);
  const [keyword, setKeyword] = useState("");
  const [debouncedKeyword] = useDebounce(
    selectedAccount?.repositorySelection === "all" ? keyword : "",
    500
  );
  const [results, setResults] = useState<SelectableRepo[] | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const abortControllerRef = useRef<AbortController | null>(null);

  const searchResults = useMemo(() => {
    if (!results) return [];
    if (selectedAccount?.repositorySelection !== "all") {
      return results.filter((result) => result.repo.toLowerCase().includes(keyword.toLowerCase()));
    }
    return results;
  }, [results, keyword, selectedAccount]);

  const displayedKeyword = useMemo(() => {
    if (selectedAccount?.repositorySelection === "all") return debouncedKeyword.trim();
    return keyword.trim();
  }, [debouncedKeyword, keyword, selectedAccount]);

  useEffect(() => {
    const fetchResults = async () => {
      if (!selectedAccount) return;

      abortControllerRef.current?.abort();
      abortControllerRef.current = new AbortController();

      setIsLoading(true);
      try {
        const results = await loadRepos(displayedKeyword);
        setResults(results);
      } catch (error: unknown) {
        if ((error as Error)?.name !== "AbortError") {
          console.error(error);
          setResults([]);
        }
      } finally {
        setIsLoading(false);
      }
    };

    fetchResults();

    return () => {
      abortControllerRef.current?.abort();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedKeyword, selectedAccount]);

  const resultsLoadingSkeleton = useMemo(() => (
    <ul>
      {[...Array(5)].map((_, index) => (
        <li key={index} className="flex gap-x-2 items-center border border-b-0 last:border-b first:rounded-t-md last:rounded-b-md px-3 py-2 text-sm">
          <Skeleton className="h-5 w-24 text-left rounded" />
          <Skeleton className="h-5 w-24 text-left rounded" />
          <Button variant="outline" size="xs" className="ml-auto" disabled>
            Open
          </Button>
        </li>
      ))}
    </ul>
  ), []);

  return (
    <div className="flex flex-col gap-y-4">
      <div className="flex w-full max-w items-center gap-x-2">
        <ButtonGroup>
          <DropdownMenuish
            accounts={accounts}
            selected={selectedAccount}
            onSelect={(account) => {
              setSelectedAccount(account);
              if (onAccountSelect) onAccountSelect(account);
            }}
            onSignOut={onSignOut}
          />
          <a
            className={cn(buttonVariants({ variant: "outline", size: "icon" }))}
            href="https://github.com/settings/applications"
            target="_blank"
            rel="noreferrer"
            aria-label="Manage authorized OAuth Apps on GitHub"
          >
            <Settings />
          </a>
        </ButtonGroup>
        <div className="relative flex-1">
          <Input
            placeholder="Search repositories by name"
            className="pl-9"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
          />
          <svg className="h-4 w-4 absolute left-3 top-1/2 -translate-y-1/2 opacity-50 pointer-events-none" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        </div>
      </div>
      {isLoading || results === null
        ? resultsLoadingSkeleton
        : searchResults.length > 0
          ? <ul>
              {searchResults.map((result) => (
                <li key={`${result.owner}/${result.repo}`} className="flex gap-x-2 items-center border border-b-0 last:border-b first:rounded-t-md last:rounded-b-md px-3 py-2 text-sm">
                  <button
                    className="truncate font-medium hover:underline"
                    onClick={() => onOpenRepo(result)}
                  >{result.repo}</button>
                  {result.private && (
                    <svg className="h-3 w-3 shrink-0 opacity-50" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
                  )}
                  {result.updatedAt &&
                    <div className="text-muted-foreground truncate">{formatDistanceToNow(new Date(result.updatedAt))} ago</div>
                  }
                  <button
                    className={cn("ml-auto", buttonVariants({ variant: "outline", size: "xs" }))}
                    onClick={() => onOpenRepo(result)}
                  >
                    Open
                  </button>
                </li>
              ))}
            </ul>
          : <div className="flex h-[206px] flex-none flex-col items-center justify-center gap-2 rounded-md border border-line bg-accent p-4 text-center">
              <p className="text-sm font-medium">No projects</p>
              <p className="text-sm text-ink-dim">No projects matched your search.</p>
            </div>
      }
    </div>
  );
}

/* Su DropdownMenu de cuentas (radix): aquí solo queda el mínimo con la
   misma pinta, porque Folio tiene una sola cuenta (el token del Device
   Flow). Si el usuario agrega más cuentas, se porta su dropdown-menu. */
function DropdownMenuish({
  accounts,
  selected,
  onSelect,
  onSignOut,
}: {
  accounts: SelectAccount[];
  selected?: SelectAccount;
  onSelect: (account: SelectAccount) => void;
  onSignOut?: () => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <Button variant="outline" onClick={() => setOpen((v) => !v)}>
        {selected && (
          <>
            <img
              className="size-6 rounded"
              src={`https://github.com/${selected.login}.png`}
              alt={`${selected.login}'s avatar`}
            />
            <span className="mr-2">{selected.login}</span>
          </>
        )}
        <svg className="ml-auto h-4 w-4 opacity-50" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>
      </Button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute left-0 top-full z-20 mt-1 min-w-full overflow-hidden rounded-md border border-line bg-popover p-1 text-popover-foreground shadow-md">
            {accounts.map((account) => (
              <button
                key={account.login}
                onClick={() => {
                  onSelect(account);
                  setOpen(false);
                }}
                className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground"
              >
                <img
                  className="size-6 rounded"
                  src={`https://github.com/${account.login}.png`}
                  alt={`${account.login}'s avatar`}
                />
                <span className="truncate">{account.login}</span>
              </button>
            ))}
            {onSignOut && (
              <>
                <div className="my-1 border-t border-line" />
                <button
                  onClick={() => {
                    setOpen(false);
                    onSignOut();
                  }}
                  className="w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground"
                >
                  Sign out
                </button>
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
}
