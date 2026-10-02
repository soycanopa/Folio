/*
 * Copiado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Source: components/repo/repo-latest.tsx
 * Adapted: next/link → button que abre el proyecto localmente.
 */
import { useState, useEffect } from "react";
import { formatDistanceToNow } from "date-fns";
import { buttonVariants } from "../ui/button";
import { cn } from "../../lib/utils";
import { getVisits } from "../../lib/tracker";
import { Skeleton } from "../ui/skeleton";

type Visit = ReturnType<typeof getVisits>[number];

interface RepoLatestProps {
  onOpenRepo: (visit: { owner: string; repo: string; branch: string }) => void;
}

export function RepoLatest({ onOpenRepo }: RepoLatestProps) {
  const [recentVisits, setRecentVisits] = useState<Visit[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const displayedVisits = recentVisits.slice(0, 3);

  useEffect(() => {
    // Only run in browser
    if (typeof window !== 'undefined') {
      const visits = getVisits();
      setRecentVisits(visits);
      setIsLoading(false);
    }
  }, []);

  if (isLoading) {
    return (
      <ul>
        {[...Array(3)].map((_, index) => (
          <li
            key={index}
            className={cn(
              "flex gap-x-2 items-center border border-b-0 last:border-b px-3 py-2 text-sm",
              index === 0 && "rounded-t-md",
              index === 2 && "rounded-b-md",
            )}
          >
            <Skeleton className="h-6 w-6 rounded" />
            <Skeleton className="h-5 w-24 rounded" />
            <Skeleton className="h-5 w-20 rounded ml-auto" />
            <Skeleton className="h-6 w-12 rounded" />
          </li>
        ))}
      </ul>
    );
  }

  if (displayedVisits.length === 0) return null;

  return (
    <ul>
      {displayedVisits.map((visit, index) => (
        <li
          key={index}
          className={cn(
            "flex gap-x-2 items-center border border-b-0 last:border-b px-3 py-2 text-sm",
            index === 0 && "rounded-t-md",
            index === displayedVisits.length - 1 && "rounded-b-md"
          )}
        >
          <img src={`https://github.com/${visit.owner}.png`} alt={visit.owner} className="h-6 w-6 rounded" />
          <button
            className="truncate font-medium hover:underline"
            onClick={() => onOpenRepo(visit)}
          >{visit.repo}</button>
          <div className="text-muted-foreground truncate">{formatDistanceToNow(new Date(visit.timestamp * 1000))} ago</div>
          <button
            className={cn("ml-auto", buttonVariants({ variant: "outline", size: "xs" }))}
            onClick={() => onOpenRepo(visit)}
          >
            Open
          </button>
        </li>
      ))}
    </ul>
  );
}
