import React from "react";
import ReactDOM from "react-dom/client";
import {
  AlertTriangle,
  RefreshCw,
  Search
} from "lucide-react";
import { Button } from "./components/ui/button";
import { Card } from "./components/ui/card";
import { Input } from "./components/ui/input";
import {
  fetchBoard,
  postMarkPrMerged,
  postRecoverTask,
  postRetireTask,
  postRetryPr,
  postStartTask,
  postSyncRun
} from "./features/symphony/api";
import { BoardGrid, SummaryStrip } from "./features/symphony/board";
import { ConfettiBurstHost, TaskDetail, TaskDetailOverlay } from "./features/symphony/detail";
import { states } from "./features/symphony/constants";
import { ControllerSidebar } from "./features/symphony/sidebar";
import type {
  BoardItem,
  BoardState,
  PrMergedResponse,
  PrRetryResponse,
  RecoveryAction
} from "./features/symphony/types";
import { cn } from "./lib/utils";
import "./styles.css";

type ConfettiBurst = {
  id: number;
  x: number;
  y: number;
};

function App() {
  const [items, setItems] = React.useState<BoardItem[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [confettiBursts, setConfettiBursts] = React.useState<ConfettiBurst[]>([]);
  const [query, setQuery] = React.useState("");
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [startingId, setStartingId] = React.useState<string | null>(null);
  const [deletingId, setDeletingId] = React.useState<string | null>(null);
  const [recoveringId, setRecoveringId] = React.useState<string | null>(null);
  const [syncingRunId, setSyncingRunId] = React.useState<string | null>(null);
  const [markingMergedRunId, setMarkingMergedRunId] = React.useState<string | null>(null);
  const [retryingPrRunId, setRetryingPrRunId] = React.useState<string | null>(null);
  const confettiBurstIdRef = React.useRef(0);
  const boardRequestIdRef = React.useRef(0);
  const selectedOpenerRef = React.useRef<HTMLElement | null>(null);
  const prefersReducedMotion = usePrefersReducedMotion();

  const loadBoard = React.useCallback(async (options?: { silent?: boolean }) => {
    const requestId = (boardRequestIdRef.current += 1);
    if (!options?.silent) {
      setLoading(true);
    }
    if (!options?.silent) {
      setError(null);
    }
    try {
      const data = await fetchBoard();
      if (requestId !== boardRequestIdRef.current) {
        return;
      }
      setItems(data.items);
    } catch (cause) {
      if (requestId === boardRequestIdRef.current && !options?.silent) {
        setError(cause instanceof Error ? cause.message : "Board request failed");
      }
    } finally {
      if (requestId === boardRequestIdRef.current && !options?.silent) {
        setLoading(false);
      }
    }
  }, []);

  React.useEffect(() => {
    void loadBoard();
  }, [loadBoard]);

  const filtered = React.useMemo(() => {
    const value = query.trim().toLowerCase();
    return items.filter(
      (item) =>
        value.length === 0 ||
        item.id.toLowerCase().includes(value) ||
        item.title.toLowerCase().includes(value)
    );
  }, [items, query]);
  const selected = selectedId ? items.find((item) => item.id === selectedId) ?? null : null;
  const counts = React.useMemo(
    () =>
      Object.fromEntries(states.map((state) => [state, items.filter((item) => item.board_state === state).length])) as
        Record<BoardState, number>,
    [items]
  );
  const activeRun = items.find((item) => item.active_run);
  const selectTask = React.useCallback((id: string) => {
    selectedOpenerRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setSelectedId(id);
  }, []);

  React.useEffect(() => {
    if (!activeRun?.active_run) {
      return;
    }
    const timer = window.setInterval(() => {
      void loadBoard({ silent: true });
    }, 1500);
    return () => window.clearInterval(timer);
  }, [activeRun?.active_run, loadBoard]);

  const clearConfettiBurst = React.useCallback((id: number) => {
    setConfettiBursts((current) => current.filter((burst) => burst.id !== id));
  }, []);

  const closeSelectedTask = React.useCallback(
    (origin?: HTMLElement) => {
      if (origin && !prefersReducedMotion) {
        const rect = origin.getBoundingClientRect();
        const burst: ConfettiBurst = {
          id: (confettiBurstIdRef.current += 1),
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2
        };
        setConfettiBursts((current) => [...current.slice(-2), burst]);
      }
      setSelectedId(null);
    },
    [prefersReducedMotion]
  );

  const startTask = React.useCallback(
    async (storyId: string) => {
      setStartingId(storyId);
      setError(null);
      try {
        await postStartTask(storyId);
        await loadBoard();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Start failed");
      } finally {
        setStartingId(null);
      }
    },
    [loadBoard]
  );

  const retireTask = React.useCallback(
    async (item: BoardItem) => {
      if (!window.confirm(`Retire ${item.id} ${item.title}? This removes it from active Ready work without deleting history.`)) {
        return;
      }
      setDeletingId(item.id);
      setError(null);
      try {
        await postRetireTask(item.id);
        setSelectedId(null);
        await loadBoard();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Delete failed");
      } finally {
        setDeletingId(null);
      }
    },
    [loadBoard]
  );

  const recoverTask = React.useCallback(
    async (storyId: string, action: RecoveryAction) => {
      if (!window.confirm(action.confirmation)) {
        return;
      }
      setRecoveringId(storyId);
      setError(null);
      try {
        await postRecoverTask(action);
        await loadBoard();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Recovery failed");
      } finally {
        setRecoveringId(null);
      }
    },
    [loadBoard]
  );

  const syncRun = React.useCallback(
    async (runId: string) => {
      setSyncingRunId(runId);
      setError(null);
      try {
        const result = await postSyncRun(runId);
        if (!result.applied) {
          setError("No new changeset was applied for that run.");
        }
        await loadBoard();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Sync failed");
      } finally {
        setSyncingRunId(null);
      }
    },
    [loadBoard]
  );

  const markPrMerged = React.useCallback(
    async (runId: string): Promise<PrMergedResponse> => {
      setMarkingMergedRunId(runId);
      setError(null);
      try {
        const result = await postMarkPrMerged(runId);
        await loadBoard();
        return result;
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Merge update failed");
        throw cause;
      } finally {
        setMarkingMergedRunId(null);
      }
    },
    [loadBoard]
  );

  const retryPr = React.useCallback(
    async (runId: string, action: RecoveryAction): Promise<PrRetryResponse> => {
      if (!window.confirm(action.confirmation)) {
        throw new Error("PR retry cancelled");
      }
      setRetryingPrRunId(runId);
      setError(null);
      try {
        const result = await postRetryPr(action);
        await loadBoard();
        return result;
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "PR retry failed");
        throw cause;
      } finally {
        setRetryingPrRunId(null);
      }
    },
    [loadBoard]
  );

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto grid w-full max-w-[1720px] grid-cols-1 gap-4 p-3 md:p-4 lg:grid-cols-[224px_minmax(0,1fr)] xl:p-6">
        <ControllerSidebar counts={counts} items={items} selectedId={selected?.id ?? null} onSelect={selectTask} />

        <div className="flex min-w-0 flex-col gap-3">
          <header className="flex flex-col gap-3 border-b border-border pb-3 xl:flex-row xl:items-end xl:justify-between">
            <div>
              <h1 className="text-[40px] font-semibold leading-none tracking-tight max-md:text-[28px]">
                Symphony work board
              </h1>
              <p className="mt-2 max-w-3xl text-base font-medium leading-6 text-muted-foreground">
                Kanban, blockers, and run logs in one focused view.
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <label className="relative block w-full sm:w-72">
                <span className="sr-only">Find task</span>
                <Search className="pointer-events-none absolute left-3 top-2.5 size-4 text-muted-foreground" />
                <Input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  className="pl-9"
                  placeholder="Find task"
                  aria-label="Find task"
                />
              </label>
              <Button variant="outline" onClick={() => void loadBoard()} disabled={loading}>
                <RefreshCw data-icon="inline-start" className={cn(loading && "motion-safe:animate-spin")} />
                Refresh
              </Button>
            </div>
          </header>

          <SummaryStrip activeRun={activeRun} counts={counts} className="order-3 md:order-none" />

          {error ? (
            <Card role="alert" className="order-2 flex items-center gap-3 border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive md:order-none">
              <AlertTriangle className="size-4 shrink-0" />
              {error}
            </Card>
          ) : null}

          <section
            id="board"
            aria-busy={loading}
            className="order-2 grid min-h-[calc(100dvh-220px)] grid-cols-1 gap-3 md:order-none"
          >
            <div className="sr-only" role="status" aria-live="polite">
              {loading
                ? "Loading Symphony board."
                : activeRun?.active_run
                  ? `Active run ${activeRun.active_run} is updating.`
                  : "Symphony board loaded."}
            </div>
            <BoardGrid items={filtered} selectedId={selected?.id ?? null} onSelect={selectTask} />
          </section>

          <ConfettiBurstHost bursts={confettiBursts} onBurstDone={clearConfettiBurst} />

          {selected ? (
            <TaskDetailOverlay restoreFocusElement={selectedOpenerRef.current} onClose={() => setSelectedId(null)}>
              <TaskDetail
                item={selected}
                startingId={startingId}
                deletingId={deletingId}
                recoveringId={recoveringId}
                syncingRunId={syncingRunId}
                markingMergedRunId={markingMergedRunId}
                retryingPrRunId={retryingPrRunId}
                onClose={closeSelectedTask}
                onStart={startTask}
                onRetire={retireTask}
                onRecover={recoverTask}
                onSync={syncRun}
                onMarkPrMerged={markPrMerged}
                onRetryPr={retryPr}
              />
            </TaskDetailOverlay>
          ) : null}

          <p className="text-xs leading-5 text-muted-foreground">
            Source: local Symphony API responses for board state, run events, review artifacts, PR status, and sync state.
          </p>
        </div>
      </div>
    </main>
  );
}

function usePrefersReducedMotion() {
  const [prefersReducedMotion, setPrefersReducedMotion] = React.useState(() =>
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );

  React.useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    function syncPreference() {
      setPrefersReducedMotion(mediaQuery.matches);
    }

    syncPreference();
    mediaQuery.addEventListener("change", syncPreference);
    return () => mediaQuery.removeEventListener("change", syncPreference);
  }, []);

  return prefersReducedMotion;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
