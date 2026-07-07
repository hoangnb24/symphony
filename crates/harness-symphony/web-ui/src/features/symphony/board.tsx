import { AlertTriangle } from "lucide-react";
import { Badge } from "../../components/ui/badge";
import { Card } from "../../components/ui/card";
import { cn } from "../../lib/utils";
import { columnId, stateIcon, states } from "./constants";
import { StatusBadge, toneForState } from "./status-badge";
import type { BoardItem, BoardState } from "./types";

export function SummaryStrip({
  activeRun,
  counts,
  className
}: {
  activeRun: BoardItem | undefined;
  counts: Record<BoardState, number>;
  className?: string;
}) {
  const metrics = [
    {
      label: "Active run",
      value: activeRun?.id ?? "none",
      detail: activeRun?.active_run ? `${activeRun.active_run} is the only task allowed in progress.` : "No active Symphony run."
    },
    {
      label: "Safe to start",
      value: `${counts.Ready} ready`,
      detail: "Ready tasks have no incomplete blockers."
    },
    {
      label: "Avoid for now",
      value: `${counts.Blocked} blocked`,
      detail: "Blocked work explains its missing dependency before action."
    },
    {
      label: "Needs decision",
      value: `${counts.Review} review`,
      detail: "Merge PR first, then approve local sync."
    }
  ];

  return (
    <section aria-label="Dashboard summary" className={cn("grid gap-2 md:grid-cols-2 xl:grid-cols-[1.2fr_.9fr_.9fr_1fr]", className)}>
      {metrics.map((metric, index) => (
        <Card key={metric.label} className={cn("rounded-md p-3", index === 0 && "bg-muted")}>
          <span className="text-xs font-bold uppercase tracking-widest text-muted-foreground">{metric.label}</span>
          <strong className="mt-1 block text-xl leading-tight">{metric.value}</strong>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">{metric.detail}</p>
        </Card>
      ))}
    </section>
  );
}

export function BoardGrid({
  items,
  selectedId,
  onSelect
}: {
  items: BoardItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="min-h-0 min-w-0 overflow-x-auto pb-2">
      <div className="grid h-[calc(100dvh-220px)] min-h-[390px] min-w-[1120px] grid-cols-[repeat(6,minmax(176px,1fr))] items-stretch gap-3 max-sm:h-auto max-sm:min-h-0 max-sm:min-w-0 max-sm:grid-cols-1">
        {states.map((state) => {
          const stateItems = items.filter((item) => item.board_state === state);
          const Icon = stateIcon[state];
          return (
            <section
              key={state}
              id={columnId(state)}
              aria-label={`${state} column`}
              className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-muted/60 max-sm:h-[min(520px,calc(100dvh-180px))] max-sm:min-h-[320px]"
            >
              <div className="flex min-h-12 items-center justify-between gap-2 border-b border-border bg-background px-3">
                <div className="flex items-center gap-2">
                  <Icon className={cn("size-4 text-muted-foreground", state === "In Progress" && "motion-safe:animate-spin")} />
                  <h2 className="text-sm font-bold">{state}</h2>
                </div>
                <StatusBadge state={state}>{stateItems.length}</StatusBadge>
              </div>
              <div aria-label={`${state} tasks`} className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
                {stateItems.map((item) => (
                  <TaskCard key={item.id} item={item} selected={item.id === selectedId} onSelect={onSelect} />
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
  const blocked = item.board_state === "Blocked";
  const attention = item.board_state === "Needs Attention";
  const done = item.board_state === "Done";

  return (
    <button
      onClick={() => onSelect(item.id)}
      className={cn(
        "block min-h-[132px] w-full min-w-0 shrink-0 overflow-hidden rounded-md border border-border bg-background p-3 text-left shadow-sm transition hover:border-primary hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        selected && "border-primary shadow-md",
        blocked && "bg-warning/10",
        attention && "bg-destructive/10",
        done && "opacity-80"
      )}
      data-testid="task-card"
    >
      <div className="flex min-w-0 items-center justify-between gap-2">
        <span className="min-w-0 truncate font-mono text-xs font-bold text-muted-foreground">{item.id}</span>
        <Badge className="max-w-[58%] shrink-0 truncate" tone={item.verify === "configured" ? toneForState(item.board_state) : "neutral"}>
          {item.board_state === "In Progress" ? "active" : item.verify}
        </Badge>
      </div>
      <h3 className="bounded-text mt-2 line-clamp-3 text-sm font-bold leading-5">{item.title}</h3>
      <p className="bounded-text mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{item.reason}</p>
      {item.failure_summary ? (
        <div className="mt-2 flex min-w-0 items-start gap-2 overflow-hidden rounded-sm border border-destructive/20 bg-destructive/10 px-2 py-1 text-xs font-semibold text-destructive">
          <AlertTriangle className="size-3 shrink-0" />
          <span className="bounded-text line-clamp-2 min-w-0">{item.failure_summary.category}</span>
        </div>
      ) : null}
      <div className="mt-3 flex min-w-0 flex-wrap gap-1 border-t border-border/70 pt-2">
        <span className="max-w-full truncate rounded-full border border-border bg-background px-2 py-0.5 text-xs font-semibold text-muted-foreground">
          {item.board_state === "Ready" ? "Start" : item.board_state === "Blocked" ? "Start disabled" : item.lane}
        </span>
        <span className="max-w-full truncate rounded-full border border-border bg-background px-2 py-0.5 text-xs font-semibold text-muted-foreground">
          {item.blockers.length > 0 ? `${item.blockers.length} blockers` : item.run_id ?? "No run"}
        </span>
      </div>
    </button>
  );
}
