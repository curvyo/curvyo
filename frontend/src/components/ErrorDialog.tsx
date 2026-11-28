import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Check } from "lucide-react";

interface ErrorDialogProps {
  title: string;
  /** The one-sentence cause, or `null` when there is nothing to show. */
  message: string | null;
  onDismiss: () => void;
}

/**
 * The blocking failure dialog this slice uses for both open errors
 * (acceptance criterion 7: a `.curvyo` that isn't a zip, is damaged, or is
 * from a newer version of the app) and save errors (a write that failed
 * must never look like it succeeded) — the same dialog "machinery", a
 * different title/message per caller.
 *
 * Looks as `docs/ui-elements.md` §19 (app tokens, one primary OK). Uses shadcn/ui `AlertDialog` rather than a toast or `Dialog` — it is the
 * primitive that does not dismiss on an outside click, which matters here
 * because this is a failure the maker must acknowledge
 * (specs/0001-project-file-foundation: "Error handling — invalid/corrupt
 * file"). For the open-error case specifically: opening this dialog never
 * depends on replacing the open project's state — the host only emits the
 * error after deciding *not* to touch it.
 */
export function ErrorDialog({ title, message, onDismiss }: ErrorDialogProps) {
  return (
    <AlertDialog
      open={message !== null}
      onOpenChange={(open) => {
        if (!open) {
          onDismiss();
        }
      }}
    >
      <AlertDialogContent className="rounded-[8px] bg-[var(--toolbar-bg)] text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)] ring-0">
        <AlertDialogHeader>
          <AlertDialogTitle className="text-sm font-semibold">{title}</AlertDialogTitle>
          <AlertDialogDescription className="text-xs text-[var(--toolbar-icon)]">
            {message}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="mx-0 mb-0 rounded-none border-t-0 bg-transparent p-0">
          <AlertDialogAction asChild>
            <Button variant="primary" icon={Check} autoFocus onClick={onDismiss}>
              OK
            </Button>
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
