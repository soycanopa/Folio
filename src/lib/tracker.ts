/*
 * Copiado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Source: lib/tracker.ts
 * Adapted: la visita registra el modo en que Folio abrió el proyecto
 * ("local" = clon en disco, "remote" = API de GitHub sin clonar) para
 * reabrirlo igual sin volver a preguntar.
 */

type RepoVisit = {
  owner: string;
  repo: string;
  branch: string;
  timestamp: number;
  mode?: 'local' | 'remote';
};

const STORAGE_KEY = 'latestVisits';
const MAX_VISITS = 5;

const trackVisit = (owner: string, repo: string, branch: string, mode: 'local' | 'remote' = 'local'): void => {
  try {
    const storedVisits = localStorage.getItem(STORAGE_KEY);
    const visits: RepoVisit[] = storedVisits ? JSON.parse(storedVisits) : [];

    const existingIndex = visits.findIndex(v =>
      v.owner.toLowerCase() === owner.toLowerCase() &&
      v.repo.toLowerCase() === repo.toLowerCase());

    const currentTime = Math.floor(Date.now() / 1000);

    if (existingIndex >= 0) {
      visits[existingIndex] = {
        owner,
        repo,
        branch,
        timestamp: currentTime,
        mode,
      };
    } else {
      visits.push({
        owner,
        repo,
        branch,
        timestamp: currentTime,
        mode,
      });
    }
    
    const updatedVisits = visits
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, MAX_VISITS);
    
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updatedVisits));
  } catch (error) {
    console.error('Failed to save recent visit to localStorage', error);
  }
}

const getVisits = (): RepoVisit[] => {
  try {
    const storedVisits = localStorage.getItem(STORAGE_KEY);
    const visits: RepoVisit[] = storedVisits ? JSON.parse(storedVisits) : [];
    return visits;
  } catch (error) {
    console.error('Failed to load recent visits from localStorage', error);
    return [];
  }
}

export { trackVisit, getVisits }