import React from "react";
import ReactDOM from "react-dom/client";
import {
  AlertTriangle,
  CheckCircle2,
  Circle,
  Clock3,
  GitPullRequestArrow,
  Loader2,
  Play,
  RefreshCw,
  Search,
  ShieldAlert
} from "lucide-react";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Card } from "./components/ui/card";
import "./styles.css";

type BoardState =
  | "Ready"
  | "Blocked"
  | "In Progress"
  | "Review"
  | "Needs Attention"
  | "Done";

type BoardItem = {
  id: string;
  title: string;
  board_state: BoardState;
  story_status: string;
  lane: string;
  verify: string;
  blockers: string[];
  unblocks: string[];
  parent_id: string | null;
  children: string[];
  hierarchy_depth: number;
  run_id: string | null;
  active_run: string | null;
  reason: string;
};

type BoardResponse = {
  items: BoardItem[];
};

type RunEvent = {
  method?: string;
  params?: unknown;
};

type EventsResponse = {
  run_id: string;
  events: RunEvent[];
};

type ReviewResponse = {
  run_id: string;
  story_id: string;
  status: string;
  outcome: string | null;
  summary: string | null;
  result: unknown | null;
  validation: unknown | null;
  changed_files: string[];
  changeset_preview: string | null;
  pr_url: string | null;
  pr_status: string;
  artifact_paths: string[];
  events: RunEvent[];
  suggested_next_action: string;
};

type SyncResponse = {
  run_id: string;
  applied: boolean;
};

type PrMergedResponse = {
  run_id: string;
  pr_status: string;
};

const states: BoardState[] = [
  "Ready",
  "Blocked",
  "In Progress",
  "Review",
  "Needs Attention",
  "Done"
];

const stateIcon = {
  Ready: Circle,
  Blocked: ShieldAlert,
  "In Progress": Loader2,
  Review: GitPullRequestArrow,
  "Needs Attention": AlertTriangle,
  Done: CheckCircle2
};

const stateTone = {
  Ready: "ready",
  Blocked: "blocked",
  "In Progress": "progress",
  Review: "review",
  "Needs Attention": "attention",
  Done: "done"
} as const;

function App() {
  const [items, setItems] = React.useState<BoardItem[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [query, setQuery] = React.useState("");
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [startingId, setStartingId] = React.useState<string | null>(null);
  const [syncingRunId, setSyncingRunId] = React.useState<string | null>(null);
  const [markingMergedRunId, setMarkingMergedRunId] = React.useState<string | null>(null);

  const loadBoard = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch("/api/board");
      if (!response.ok) {
        throw new Error(`Board request failed (${response.status})`);
      }
      const data = (await response.json()) as BoardResponse;
      setItems(data.items);
      setSelectedId((current) => current ?? data.items[0]?.id ?? null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Board request failed");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void loadBoard();
  }, [loadBoard]);

  const filtered = items.filter((item) => {
    const value = query.trim().toLowerCase();
    return (
      value.length === 0 ||
      item.id.toLowerCase().includes(value) ||
      item.title.toLowerCase().includes(value)
    );
  });
  const selected = items.find((item) => item.id === selectedId) ?? filtered[0] ?? null;

  const startTask = React.useCallback(
    async (storyId: string) => {
      setStartingId(storyId);
      setError(null);
      try {
        const response = await fetch(`/api/tasks/${encodeURIComponent(storyId)}/start`, {
          method: "POST"
        });
        if (!response.ok) {
          const body = (await response.json().catch(() => null)) as { error?: string } | null;
          throw new Error(body?.error ?? `Start failed (${response.status})`);
        }
        await loadBoard();
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Start failed");
      } finally {
        setStartingId(null);
      }
    },
    [loadBoard]
  );

  const syncRun = React.useCallback(
    async (runId: string) => {
      setSyncingRunId(runId);
      setError(null);
      try {
        const response = await fetch(`/api/runs/${encodeURIComponent(runId)}/sync`, {
          method: "POST"
        });
        if (!response.ok) {
          const body = (await response.json().catch(() => null)) as { error?: string } | null;
          throw new Error(body?.error ?? `Sync failed (${response.status})`);
        }
        const result = (await response.json()) as SyncResponse;
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

  const markPrMerged = React.useCallback(async (runId: string) => {
    setMarkingMergedRunId(runId);
    setError(null);
    try {
      const response = await fetch(`/api/runs/${encodeURIComponent(runId)}/pr-merged`, {
        method: "POST"
      });
      if (!response.ok) {
        const body = (await response.json().catch(() => null)) as { error?: string } | null;
        throw new Error(body?.error ?? `Merge update failed (${response.status})`);
      }
      await response.json() as PrMergedResponse;
      await loadBoard();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Merge update failed");
    } finally {
      setMarkingMergedRunId(null);
    }
  }, [loadBoard]);

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex w-full max-w-[1600px] flex-col gap-4 px-4 py-4 md:px-6">
        <header className="flex flex-col gap-3 border-b border-border pb-4 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <h1 className="text-xl font-semibold">Symphony Controller</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Local task board for Harness Symphony runs.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <label className="relative block">
              <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                className="h-9 w-64 rounded-md border border-input bg-background pl-9 pr-3 text-sm outline-none focus:ring-2 focus:ring-ring"
                placeholder="Find task"
              />
            </label>
            <Button variant="outline" onClick={() => void loadBoard()} disabled={loading}>
              <RefreshCw className={loading ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
              Refresh
            </Button>
          </div>
        </header>

        {error ? (
          <Card className="flex items-center gap-3 border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
            <AlertTriangle className="h-4 w-4" />
            {error}
          </Card>
        ) : null}

        <section className="grid min-h-[calc(100vh-132px)] gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
          <BoardGrid items={filtered} selectedId={selected?.id ?? null} onSelect={setSelectedId} />
          <TaskDetail
            item={selected}
            startingId={startingId}
            syncingRunId={syncingRunId}
            markingMergedRunId={markingMergedRunId}
            onStart={startTask}
            onSync={syncRun}
            onMarkPrMerged={markPrMerged}
          />
        </section>
      </div>
    </main>
  );
}

function BoardGrid({
  items,
  selectedId,
  onSelect
}: {
  items: BoardItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6">
      {states.map((state) => {
        const stateItems = items.filter((item) => item.board_state === state);
        const Icon = stateIcon[state];
        return (
          <section key={state} className="flex min-h-0 flex-col rounded-lg border border-border bg-muted/35">
            <div className="flex h-12 items-center justify-between border-b border-border px-3">
              <div className="flex items-center gap-2">
                <Icon className="h-4 w-4 text-muted-foreground" />
                <h2 className="text-sm font-semibold">{state}</h2>
              </div>
              <Badge tone={stateTone[state]}>{stateItems.length}</Badge>
            </div>
            <div className="flex flex-1 flex-col gap-2 overflow-auto p-2">
              {stateItems.map((item) => (
                <TaskCard
                  key={item.id}
                  item={item}
                  selected={item.id === selectedId}
                  onSelect={onSelect}
                />
              ))}
              {stateItems.length === 0 ? (
                <div className="flex min-h-24 items-center justify-center rounded-md border border-dashed border-border px-3 text-center text-xs text-muted-foreground">
                  No tasks
                </div>
              ) : null}
            </div>
          </section>
        );
      })}
    </div>
  );
}

function TaskCard({
  item,
  selected,
  onSelect
}: {
  item: BoardItem;
  selected: boolean;
  onSelect: (id: string) => void;
}) {
  return (
    <button
      onClick={() => onSelect(item.id)}
      className={[
        "min-h-28 rounded-md border bg-card p-3 text-left shadow-sm transition-colors hover:border-primary/60",
        selected ? "border-primary ring-2 ring-ring" : "border-border"
      ].join(" ")}
    >
      <div className="flex items-start justify-between gap-2">
        <span className="text-xs font-semibold text-muted-foreground">{item.id}</span>
        <Badge tone={item.verify === "configured" ? "ready" : "neutral"}>{item.verify}</Badge>
      </div>
      <h3 className="mt-2 line-clamp-3 text-sm font-semibold leading-5">{item.title}</h3>
      {item.parent_id ? (
        <p className="mt-2 text-xs text-muted-foreground">Under {item.parent_id}</p>
      ) : null}
      <p className="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{item.reason}</p>
    </button>
  );
}

function TaskDetail({
  item,
  startingId,
  syncingRunId,
  markingMergedRunId,
  onStart,
  onSync,
  onMarkPrMerged
}: {
  item: BoardItem | null;
  startingId: string | null;
  syncingRunId: string | null;
  markingMergedRunId: string | null;
  onStart: (storyId: string) => Promise<void>;
  onSync: (runId: string) => Promise<void>;
  onMarkPrMerged: (runId: string) => Promise<void>;
}) {
  const [events, setEvents] = React.useState<RunEvent[]>([]);
  const [review, setReview] = React.useState<ReviewResponse | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;

    async function loadEvents() {
      const runId = item?.active_run ?? item?.run_id;
      if (!runId) {
        setEvents([]);
        return;
      }
      try {
        const response = await fetch(`/api/runs/${encodeURIComponent(runId)}/events`);
        if (response.ok) {
          const data = (await response.json()) as EventsResponse;
          if (!cancelled) {
            setEvents(data.events);
          }
        }
      } finally {
        if (!cancelled) {
          timer = window.setTimeout(loadEvents, 2000);
        }
      }
    }

    void loadEvents();
    return () => {
      cancelled = true;
      if (timer) {
        window.clearTimeout(timer);
      }
    };
  }, [item?.active_run, item?.run_id]);

  React.useEffect(() => {
    let cancelled = false;
    const runId = item?.run_id ?? item?.active_run;
    if (!runId || !["Review", "Needs Attention", "Done"].includes(item?.board_state ?? "")) {
      setReview(null);
      return;
    }
    const reviewRunId = runId;

    async function loadReview() {
      try {
        const response = await fetch(`/api/runs/${encodeURIComponent(reviewRunId)}/review`);
        if (response.ok) {
          const data = (await response.json()) as ReviewResponse;
          if (!cancelled) {
            setReview(data);
          }
        }
      } catch {
        if (!cancelled) {
          setReview(null);
        }
      }
    }

    void loadReview();
    return () => {
      cancelled = true;
    };
  }, [item?.active_run, item?.board_state, item?.run_id]);

  if (!item) {
    return (
      <aside className="rounded-lg border border-border bg-card p-4">
        <p className="text-sm text-muted-foreground">No task selected.</p>
      </aside>
    );
  }

  const isReady = item.board_state === "Ready";
  const isStarting = startingId === item.id;

  return (
    <aside className="rounded-lg border border-border bg-card">
      <div className="border-b border-border p-4">
        <div className="flex items-center justify-between gap-3">
          <Badge tone={stateTone[item.board_state]}>{item.board_state}</Badge>
          <span className="text-xs text-muted-foreground">{item.id}</span>
        </div>
        <h2 className="mt-3 text-lg font-semibold leading-6">{item.title}</h2>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">{item.reason}</p>
      </div>

      <div className="space-y-5 p-4">
        <div className="grid grid-cols-2 gap-3 text-sm">
          <Field label="Story" value={item.story_status} />
          <Field label="Lane" value={item.lane} />
          <Field label="Proof" value={item.verify} />
          <Field label="Run" value={item.run_id ?? item.active_run ?? "none"} />
        </div>

        <HierarchyBlock item={item} />
        <ListBlock title="Blocked by" values={item.blockers} empty="No blockers" />
        <ListBlock title="Unblocks" values={item.unblocks} empty="No dependent tasks" />

        {review ? (
          <ReviewPanel
            review={review}
            syncing={syncingRunId === review.run_id}
            markingMerged={markingMergedRunId === review.run_id}
            onSync={onSync}
            onMarkPrMerged={onMarkPrMerged}
          />
        ) : null}
        {item.active_run || item.run_id ? <EventLog events={review?.events ?? events} /> : null}

        <div className="flex items-center gap-2 border-t border-border pt-4">
          <Button
            disabled={!isReady || isStarting}
            title={isReady ? "Start task" : "Blocked tasks cannot start"}
            onClick={() => void onStart(item.id)}
          >
            {isStarting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
            Start
          </Button>
          <Button variant="outline">
            <Clock3 className="h-4 w-4" />
            Details
          </Button>
        </div>
      </div>
    </aside>
  );
}

function HierarchyBlock({ item }: { item: BoardItem }) {
  return (
    <div>
      <h3 className="text-sm font-semibold">Hierarchy</h3>
      <div className="mt-2 grid grid-cols-2 gap-2 text-sm">
        <Field label="Parent" value={item.parent_id ?? "top level"} />
        <Field label="Depth" value={String(item.hierarchy_depth)} />
      </div>
      <div className="mt-2">
        <ListBlock title="Child tasks" values={item.children} empty="No child tasks" />
      </div>
    </div>
  );
}

function ReviewPanel({
  review,
  syncing,
  markingMerged,
  onSync,
  onMarkPrMerged
}: {
  review: ReviewResponse;
  syncing: boolean;
  markingMerged: boolean;
  onSync: (runId: string) => Promise<void>;
  onMarkPrMerged: (runId: string) => Promise<void>;
}) {
  const canMarkMerged = review.pr_status === "created" && review.pr_url !== null;
  const canSync = review.pr_status === "merged" && review.status === "completed";

  return (
    <div className="space-y-4 rounded-md border border-border p-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">Review</h3>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            {review.suggested_next_action}
          </p>
        </div>
        <Badge tone={review.pr_status === "created" ? "review" : "attention"}>
          {review.pr_status}
        </Badge>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <Field label="Outcome" value={review.outcome ?? "unknown"} />
        <Field label="Status" value={review.status} />
      </div>

      {review.pr_url ? (
        <a
          className="block break-all rounded-md border border-border px-3 py-2 text-sm text-primary hover:bg-accent"
          href={review.pr_url}
          target="_blank"
          rel="noreferrer"
        >
          {review.pr_url}
        </a>
      ) : null}

      {review.summary ? <TextBlock title="Summary" text={review.summary} /> : null}
      {review.validation ? (
        <TextBlock title="Validation" text={JSON.stringify(review.validation, null, 2)} />
      ) : null}
      <ListBlock title="Changed files" values={review.changed_files} empty="No changed files listed" />
      {review.changeset_preview ? (
        <TextBlock title="Changeset" text={review.changeset_preview} />
      ) : null}
      <ListBlock title="Artifacts" values={review.artifact_paths} empty="No artifacts found" />

      <div className="flex border-t border-border pt-3">
        <Button
          className="mr-2"
          variant="outline"
          disabled={!canMarkMerged || markingMerged}
          onClick={() => void onMarkPrMerged(review.run_id)}
        >
          {markingMerged ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <GitPullRequestArrow className="h-4 w-4" />
          )}
          Mark Merged
        </Button>
        <Button disabled={!canSync || syncing} onClick={() => void onSync(review.run_id)}>
          {syncing ? <Loader2 className="h-4 w-4 animate-spin" /> : <CheckCircle2 className="h-4 w-4" />}
          Approve Sync
        </Button>
      </div>
    </div>
  );
}

function TextBlock({ title, text }: { title: string; text: string }) {
  return (
    <div>
      <h3 className="text-sm font-semibold">{title}</h3>
      <pre className="mt-2 max-h-52 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/50 p-3 text-xs leading-5">
        {text}
      </pre>
    </div>
  );
}

function EventLog({ events }: { events: RunEvent[] }) {
  const recent = events.slice(-8).reverse();

  return (
    <div>
      <h3 className="text-sm font-semibold">Events</h3>
      <div className="mt-2 max-h-52 overflow-auto rounded-md border border-border">
        {recent.length > 0 ? (
          recent.map((event, index) => (
            <div
              key={`${event.method ?? "event"}-${index}`}
              className="flex min-h-9 items-center border-b border-border px-3 text-xs last:border-b-0"
            >
              <span className="font-medium">{event.method ?? "event"}</span>
            </div>
          ))
        ) : (
          <div className="flex min-h-12 items-center px-3 text-sm text-muted-foreground">
            No events yet
          </div>
        )}
      </div>
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 break-words text-sm font-medium">{value}</div>
    </div>
  );
}

function ListBlock({
  title,
  values,
  empty
}: {
  title: string;
  values: string[];
  empty: string;
}) {
  return (
    <div>
      <h3 className="text-sm font-semibold">{title}</h3>
      <div className="mt-2 flex min-h-9 flex-wrap gap-2">
        {values.length > 0 ? (
          values.map((value) => (
            <Badge key={value} tone="neutral">
              {value}
            </Badge>
          ))
        ) : (
          <span className="text-sm text-muted-foreground">{empty}</span>
        )}
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
