import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import * as ToastPrimitive from "@radix-ui/react-toast";
import clsx from "clsx";

type ToastTone = "info" | "success" | "error";

interface ToastMessage {
  id: number;
  title: string;
  tone: ToastTone;
}

interface ToastContextValue {
  notify: (title: string, tone?: ToastTone) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const toneStyles: Record<ToastTone, string> = {
  info: "border-line bg-surface text-ink",
  success: "border-line bg-surface text-ink",
  error: "border-risk-critical bg-surface text-risk-critical",
};

let nextId = 1;

/**
 * Global toast notifications (§10: "non-blocking, auto-dismiss"). Wrap the
 * app once near the root; any screen calls `useToast().notify(...)`.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [messages, setMessages] = useState<ToastMessage[]>([]);

  const notify = useCallback((title: string, tone: ToastTone = "info") => {
    const id = nextId++;
    setMessages((current) => [...current, { id, title, tone }]);
  }, []);

  const dismiss = useCallback((id: number) => {
    setMessages((current) => current.filter((message) => message.id !== id));
  }, []);

  const value = useMemo(() => ({ notify }), [notify]);

  return (
    <ToastContext.Provider value={value}>
      <ToastPrimitive.Provider swipeDirection="right">
        {children}
        {messages.map((message) => (
          <ToastPrimitive.Root
            key={message.id}
            duration={4000}
            onOpenChange={(open) => {
              if (!open) dismiss(message.id);
            }}
            className={clsx(
              "rounded-lg border px-4 py-3 text-sm shadow-md",
              "data-[state=open]:animate-in data-[state=open]:fade-in",
              toneStyles[message.tone],
            )}
          >
            <ToastPrimitive.Title>{message.title}</ToastPrimitive.Title>
          </ToastPrimitive.Root>
        ))}
        <ToastPrimitive.Viewport className="fixed bottom-6 right-6 z-50 flex w-80 flex-col gap-2 outline-none" />
      </ToastPrimitive.Provider>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}
