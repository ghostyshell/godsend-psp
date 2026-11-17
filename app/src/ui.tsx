// Shared UI primitives for the GODsend PSP shell.

import { ReactNode, useEffect, useState } from "react";
import { api } from "./api";
import type { Job } from "./types";

export function fmtBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const value = bytes / Math.pow(1024, i);
  return `${i === 0 ? bytes : value.toFixed(1)} ${units[i]}`;
}

/** Clock-style remaining time: `1:20`, `1:02:03`. */
function fmtEta(secs: number): string {
  const hours = Math.floor(secs / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  const rest = Math.floor(secs % 60);
  if (hours > 0) {
    return `${hours}:${String(mins).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
  }
  return `${mins}:${String(rest).padStart(2, "0")}`;
}

/**
 * One line for a running job: percent, bytes done of total, live speed and
 * time left. Falls back to the average rate from elapsed time when the
 * transfer cannot report a live speed, and to nothing when total is unknown.
 */
export function progressText(job: Job): string {
  if (!job.total_bytes) return "";
  const done = Math.min(job.done_bytes, job.total_bytes);
  const pct = Math.min(100, Math.round((done / job.total_bytes) * 100));
  const parts = [`${pct}% - ${fmtBytes(done)} of ${fmtBytes(job.total_bytes)}`];
  const elapsed = Math.max(1, Math.floor(Date.now() / 1000) - job.started_at);
  const speed = job.speed_bps || (done > 0 ? Math.round(done / elapsed) : 0);
  if (speed > 0) {
    parts.push(`${fmtBytes(speed)}/s`);
    const eta = job.eta_secs || Math.round((job.total_bytes - done) / speed);
    if (eta > 0 && done < job.total_bytes) {
      parts.push(`${fmtEta(eta)} left`);
    }
  }
  return parts.join(" - ");
}

export function Chip({
  tone = "neutral",
  live = false,
  children,
  title,
  className,
}: {
  tone?: "neutral" | "good" | "warn" | "bad" | "laser";
  live?: boolean;
  children: ReactNode;
  title?: string;
  className?: string;
}) {
  return (
    <span
      className={`chip ${tone} ${live ? "live" : ""}${className ? ` ${className}` : ""}`}
      title={title}
    >
      <span className="dot" aria-hidden="true" />
      {children}
    </span>
  );
}

export function Btn({
  children,
  onClick,
  variant = "ghost",
  small = false,
  disabled,
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "ghost" | "primary" | "danger" | "outline";
  small?: boolean;
  disabled?: boolean;
  title?: string;
}) {
  return (
    <button className={`btn ${variant} ${small ? "small" : ""}`} onClick={onClick} disabled={disabled} title={title}>
      {children}
    </button>
  );
}

export function Seg<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  ariaLabel: string;
}) {
  return (
    <div className="seg" role="tablist" aria-label={ariaLabel}>
      {options.map((opt) => (
        <button
          key={opt.value}
          role="tab"
          aria-selected={value === opt.value}
          className={value === opt.value ? "on" : ""}
          onClick={() => onChange(opt.value)}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

export function Bar({
  value,
  max,
  live = false,
}: {
  value: number;
  max: number;
  live?: boolean;
}) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div
      className={`bar ${live ? "live" : ""}`}
      role="progressbar"
      aria-valuenow={pct}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div className="fill" style={{ width: `${pct}%` }} />
    </div>
  );
}

export function Empty({ title, hint, action }: { title: string; hint?: string; action?: ReactNode }) {
  return (
    <div className="empty">
      <div className="empty-title">{title}</div>
      {hint ? <div className="empty-hint">{hint}</div> : null}
      {action ? <div className="empty-action">{action}</div> : null}
    </div>
  );
}

export function Eyebrow({ children }: { children: ReactNode }) {
  return <div className="eyebrow">{children}</div>;
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
}) {
  return (
    <button
      className={`switch ${checked ? "on" : ""}`}
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
    >
      <span className="knob" aria-hidden="true" />
      <span className="switch-label">{label}</span>
    </button>
  );
}

/** Plays the game's own ICON1.PMF animation over the cover on hover. */
export function GameHover({ id }: { id: string }) {
  const [url, setUrl] = useState<string | null>(null);
  const [hover, setHover] = useState(false);

  useEffect(() => {
    if (hover && url === null) {
      let alive = true;
      void api
        .animation(id)
        .then((value) => {
          if (alive) setUrl(value);
        })
        .catch(() => setUrl(""));
      return () => {
        alive = false;
      };
    }
    return undefined;
  }, [hover, id, url]);

  return (
    <div
      className="hover-anim"
      aria-hidden="true"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
    >
      {hover && url ? <video src={url} autoPlay muted loop playsInline /> : null}
    </div>
  );
}

export function confirmDialog(title: string, body: string): boolean {
  // eslint-disable-next-line no-alert
  return window.confirm(`${title}\n\n${body}`);
}

export function toastText(text: string): string {
  return text;
}
