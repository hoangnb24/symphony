import * as React from "react";
import { cn } from "../../lib/utils";

export type BadgeTone = "neutral" | "muted" | "success" | "info" | "accent" | "danger" | "complete";

const tones: Record<BadgeTone, string> = {
  neutral: "border-border bg-muted text-muted-foreground",
  muted: "border-zinc-300 bg-zinc-100 text-zinc-700",
  success: "border-emerald-200 bg-emerald-50 text-emerald-800",
  info: "border-blue-200 bg-blue-50 text-blue-800",
  accent: "border-violet-200 bg-violet-50 text-violet-800",
  danger: "border-red-200 bg-red-50 text-red-800",
  complete: "border-teal-200 bg-teal-50 text-teal-800"
};

export function Badge({
  className,
  tone = "neutral",
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { tone?: BadgeTone }) {
  return (
    <span
      className={cn(
        "inline-flex min-h-6 items-center rounded-md border px-2 py-0.5 text-xs font-medium",
        tones[tone],
        className
      )}
      {...props}
    />
  );
}
