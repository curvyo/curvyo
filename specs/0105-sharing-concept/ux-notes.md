# UX notes: the workspace is the only unit of sharing

Implementer for the ux-engineer, 2026-11-09 (Proposal, to be confirmed at the UX review). Layout
and tokens as `docs/design-system.md` ("Share tab"). No explanatory sentences: labels, states,
real errors and the one confirm the spec asks for.

## Share tab, top to bottom

1. **Server** field with the reachability line (`Reaching...`, `Connected`, `Cannot reach
   <server>.` with Try again); re-checked every 5 s while the tab is shown and nothing is linked.
   With a link the field becomes the server's name. **Name shown to others** below it.
2. **Workspace block** (a workspace is open): name left, state right (`Local`, `Synced`,
   `Shared`, `Sharing...`). No Sync switch (customer feedback on PR #298). **Invite person**: in
   a Local workspace one inline confirm, the sentence "Names, folders and files leave this device
   encrypted." with **Invite person** and Cancel, which starts sharing and then shows the link; in
   a synced one it writes the link at once. Greyed (40 %, `aria-disabled`) whenever it cannot work,
   the reason as tooltip and description (texts from `curvyo-ui-core::sharing_text`). On a device
   in a device group the `0096` switch **Sync to my devices** (on at once, off armed). People list
   below; the only participant left has Stop sharing.
3. **No workspace open**: the line "Sharing works on workspaces." and, for a saved file on the
   desktop, **Make a workspace for this file**, whose inline confirm reads "New folder <path>.
   <file> moves into it." with **Make workspace** and Cancel. The browser shows "Sharing works on
   workspaces, made in the desktop app." only. A greyed Invite person below says "Open a
   workspace first." (the browser: "Invite people from the desktop app.").
4. **My devices**: the list and **Add device**; no Join button of its own.
5. **Join…**: one field "Link or code". A `2.` link shows the name field, Choose folder... and
   Join (desktop; the browser says "Open this link in the desktop app."); a `1.` link shows its
   refusal as an alert; a device code shows the device name, the synced folder and Connect, then
   the check words of `0096`.

## People

A person row is a disclosure button with the line "Marc (you) · Admin · 2 devices", expanded by
default (Proposal); its device rows are the `0097` rows (role slot, action slot, armed acts). A
device without a label is "A device".

## Tabs and focus

No action of the Share tab changes the selected tab: Make a workspace, Sync to my devices, Invite person,
Join and Add device keep the Share tab selected and the open document open. The File tab shows
"Shared through workspace <name>" under the workspace header for an open document of a synced or
shared workspace.

## Notices

Opening a file shared on its own by an older build shows one canvas notice: "This document was
shared on its own. Documents are now shared in a workspace; it opens as a local copy."
