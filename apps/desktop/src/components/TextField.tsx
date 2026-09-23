import { forwardRef, useId, type InputHTMLAttributes } from "react";
import * as Label from "@radix-ui/react-label";
import clsx from "clsx";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: string | undefined;
  hint?: string | undefined;
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(function TextField(
  { label, error, hint, id, className, ...props },
  ref,
) {
  const generatedId = useId();
  const inputId = id ?? generatedId;
  const hintId = hint ? `${inputId}-hint` : undefined;
  const errorId = error ? `${inputId}-error` : undefined;

  return (
    <div className="flex flex-col gap-1.5">
      <Label.Root htmlFor={inputId} className="text-sm font-medium text-ink">
        {label}
      </Label.Root>
      <input
        ref={ref}
        id={inputId}
        aria-describedby={clsx(hintId, errorId) || undefined}
        aria-invalid={Boolean(error)}
        className={clsx(
          "rounded-lg border bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-muted",
          "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
          error ? "border-risk-critical" : "border-line",
          className,
        )}
        {...props}
      />
      {hint && !error && (
        <p id={hintId} className="text-xs text-ink-muted">
          {hint}
        </p>
      )}
      {error && (
        <p id={errorId} className="text-xs text-risk-critical">
          {error}
        </p>
      )}
    </div>
  );
});
