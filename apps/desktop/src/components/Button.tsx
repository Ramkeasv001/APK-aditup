import { forwardRef, type ButtonHTMLAttributes } from "react";
import clsx from "clsx";

export type ButtonVariant = "primary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
}

const base =
  "inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-medium " +
  "transition-colors disabled:opacity-50 disabled:pointer-events-none " +
  "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";

const variants: Record<ButtonVariant, string> = {
  primary: "bg-accent-strong text-on-accent hover:opacity-90",
  ghost: "border border-line text-ink hover:bg-surface-alt",
  danger: "bg-risk-critical text-white hover:opacity-90",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "primary", className, ...props },
  ref,
) {
  return <button ref={ref} className={clsx(base, variants[variant], className)} {...props} />;
});
