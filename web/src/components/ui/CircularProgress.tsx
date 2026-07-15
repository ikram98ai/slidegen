import React from "react";

interface CircularProgressProps {
  /** Units of work completed so far. */
  processed: number;
  /** Total units of work; the ring fills with processed/total. */
  total: number;
  /** Outer diameter in pixels. */
  size?: number;
  /** Center text: page fraction ("12/20") or percentage ("60%"). */
  label?: "fraction" | "percent";
}

export const CircularProgress: React.FC<CircularProgressProps> = ({
  processed,
  total,
  size = 36,
  label = "fraction",
}) => {
  const strokeWidth = Math.max(3, size / 12);
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const fraction = total > 0 ? Math.min(processed / total, 1) : 0;
  const dashOffset = circumference * (1 - fraction);

  const text =
    label === "percent"
      ? `${Math.round(fraction * 100)}%`
      : `${processed}/${total}`;

  return (
    <div
      className="relative inline-flex items-center justify-center"
      style={{ width: size, height: size }}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={total}
      aria-valuenow={processed}
      title={`${processed} of ${total} pages processed`}
    >
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          strokeWidth={strokeWidth}
          className="stroke-blue-100"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={dashOffset}
          className={`stroke-blue-500 transition-[stroke-dashoffset] duration-500 ${
            fraction >= 1 ? "animate-pulse" : ""
          }`}
        />
      </svg>
      <span
        className="absolute font-semibold text-blue-600 tabular-nums"
        style={{ fontSize: Math.max(8, size * 0.26) }}
      >
        {text}
      </span>
    </div>
  );
};
