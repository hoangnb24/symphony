import { Badge, type BadgeTone } from "../../components/ui/badge";
import type { ReactNode } from "react";
import type { BoardState } from "./types";

const stateTone: Record<BoardState, BadgeTone> = {
  Ready: "success",
  Blocked: "muted",
  "In Progress": "info",
  Review: "accent",
  "Needs Attention": "danger",
  Done: "complete"
};

export function StatusBadge({
  state,
  children,
  className
}: {
  state: BoardState;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <Badge tone={stateTone[state]} className={className}>
      {children ?? state}
    </Badge>
  );
}

export function toneForState(state: BoardState): BadgeTone {
  return stateTone[state];
}
