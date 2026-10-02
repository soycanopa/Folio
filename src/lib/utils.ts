/*
 * Ported from Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Standard shadcn/ui class merge helper.
 */
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
