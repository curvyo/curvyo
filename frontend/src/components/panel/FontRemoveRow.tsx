// The Remove row an expanded Curvyo font gains in the Fonts tab (`specs/0075-font-library`
// criterion 25; UX notes §3), with its inline confirm.

import { OUTLINE_BUTTON } from "@/components/panel/FormatForm";
import { InlineConfirm } from "@/components/panel/FormatList";

interface RemoveRowProps {
  rowKey: string;
  focused: boolean;
  confirming: boolean;
  prompt: string;
  style: React.CSSProperties;
  onFocusRow: () => void;
  onAsk: () => void;
  onCancel: () => void;
  onRemove: () => void;
  onReturnFocus: () => void;
}

/** The Remove row of a Curvyo font: a button, replaced by the inline confirm when pressed. */
export function RemoveRow({
  rowKey,
  focused,
  confirming,
  prompt,
  style,
  onFocusRow,
  onAsk,
  onCancel,
  onRemove,
  onReturnFocus,
}: RemoveRowProps) {
  return (
    <div role="treeitem" aria-level={2} data-row-key={rowKey} style={style} className="pl-5" tabIndex={-1} onFocus={onFocusRow}>
      {confirming ? (
        <div onKeyDown={(event) => event.stopPropagation()}>
          <InlineConfirm prompt={prompt} action="Remove" onCancel={onCancel} onDelete={onRemove} onReturnFocus={onReturnFocus} />
        </div>
      ) : (
        <button
          type="button"
          tabIndex={focused ? 0 : -1}
          onClick={onAsk}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              event.stopPropagation();
              onAsk();
            }
          }}
          className={`${OUTLINE_BUTTON} w-full !text-[var(--destructive)]`}
        >
          Remove from Curvyo fonts
        </button>
      )}
    </div>
  );
}
