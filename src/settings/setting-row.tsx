import type { ReactNode } from "react";

/** A titled group of rows, like a macOS System Settings section. */
export function SettingGroup({
  title,
  footnote,
  children,
}: {
  title: string;
  footnote?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="group">
      <h2 className="group-title">{title}</h2>
      <div className="group-card">{children}</div>
      {footnote && <p className="group-footnote">{footnote}</p>}
    </section>
  );
}

/** Label (+ optional hint) on the left, control on the right — or below the
 * label when `stacked` (wide controls such as a key input). */
export function SettingRow({
  label,
  htmlFor,
  hint,
  stacked = false,
  children,
}: {
  label: string;
  htmlFor?: string;
  hint?: ReactNode;
  stacked?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={stacked ? "row stacked" : "row"}>
      <div className="row-text">
        {htmlFor ? (
          <label className="row-label" htmlFor={htmlFor}>
            {label}
          </label>
        ) : (
          <span className="row-label">{label}</span>
        )}
        {hint && <div className="row-hint">{hint}</div>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}
