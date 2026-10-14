# History branches: undone steps as a branch, versions to look at or go to, clone of any state

Status: Ready (2026-10-10). The customer accepted ADR 0014 (`docs/adr/0014-history-undo-and-branches.md`) and every default of the open questions below on 2026-10-10 (Preview by default, Question 2 A). `adrs.md` and the UX notes exist; the criteria are complete and testable. The build starts once `0020` and `0041` have merged. Changed by the customer feedback of 2026-10-14 on PR #119 (section below): criteria 10, 14 to 19, 21 and 31 changed in substance, 1, 2, 26, 30 and 33 in wording; criteria 11, 12 and 13 withdrawn.
Priority: Should
Origin: Customer (specification of 2026-10-10, final: steps carry ids like git; undo two steps and then do something makes the undone steps a branch, no linear redo until the conflicting step is taken back; branches shown in the history; a clone of the state at any point; clicking back and forth to change the version of an object, or a preview in the history window, perhaps a slider between live and preview; in live mode a switch to restore the state from before the clicking). Requirement R-HIST-003. The customer is not sure about live versus preview; the choices below are my proposals and are marked.

## User value

As a maker I want the steps I took back not to vanish when I try something else, to see them as a branch next to the new line of work, to look at an earlier version of an object or put the object back to it, and to make a copy of any earlier state, so that I can explore two ideas for one part and keep both.

**Reference tools.** Git: steps have ids, a new commit after going back makes a branch, `checkout` looks at an old state, `revert` and `reset` change it. Photoshop's History panel has "Allow non-linear history" and snapshots; Vim and Emacs have undo trees. Inkscape, LightBurn and Illustrator drop the redo steps as soon as the maker does something new, which is exactly what the customer calls painful. What we do: the dropped steps stay as a branch, the list says where it forked, and a version can be looked at without changing anything.

## Words used below

All terms of `0020-undo-redo` and `0041-object-history`. In addition:

- **Fork point:** the step after which two lines of work continue. **Main line:** the steps that make up the current state. **Branch:** a run of steps that were taken back (by user undo, object undo or applying a version) and then not brought back before a newer step on the same scope. A branch cannot be redone linearly.
- **Scope:** the Document scope (the maker's own timeline) or the Object scope of one object (`0041`). Branches exist in each, and are derived from the same log.
- **Version:** the state after a step. "Version a1b2c3d" is the state right after step a1b2c3d, for an object (its fields) or, in the Document scope, for the whole document.
- **Preview:** looking at a version without changing the document.
- **Apply a version:** Enter or a double-click on a row. It writes one step "Go to version <id>" that puts the object (Object scope) or the whole document (Document scope) back to that version.

## Customer feedback 2026-10-14

The customer tried PR #119 and decided (decisions of the architect: `adrs.md`, note "customer feedback 2026-10-14", and the amendment of 2026-10-14 to ADR 0014 §8):

1. **Document scope:** the preview stays. Enter or a double-click on a row applies that version as one step that Ctrl+Z takes back. There is no Apply button.
2. **Object scope:** Live mode and the Mode switch are dropped ("das live ist käse, ich mache nur Preview": the live mode is rubbish, I only use Preview). Exploring and "Restore previous state" go too. Only Preview remains. While it shows, the object's current drawing is hidden and only the previewed version is drawn. Apply works as in the Document scope; Escape ends the preview and changes nothing.
3. **Texts removed, nothing in their place:** the caption "A click shows the version. Nothing changes." (with the other Mode captions) and the line "The history is saved in the project file." (`0020` criterion 45). How to preview and apply is said only in each row's tooltip and accessible name (criterion 14).

## Acceptance criteria

### The branch model

1. **Fork.** Given an object O with applied steps s1, s2, s3, when the maker presses Ctrl+Z twice (s3 and s2 are taken back) and then makes a new step s4 on O, then s2 and s3 are a branch from the fork point s1, and s4 continues the main line. Ctrl+Shift+Z says "Nothing to redo." The same happens with object undo (`0041`) and with applying a version (criterion 10). Test with `document.json` equality: the state after s4 is s1's state plus s4; s2 and s3 remain in the log and in the list.
2. **Taking back the conflict step.** Given the fork of criterion 1, when the maker presses Ctrl+Z (s4 is taken back), then the document is at s1's state again, and Ctrl+Shift+Z brings back **s4** (the branch most recently left), not s2 (Question 1). The old branch is reachable through the list (criteria 10 and 22).
3. Given a branch, then it is identified by the id of its newest step; the list names it "Branch from a1b2c3d, 2 steps" (`a1b2c3d` the fork point, the step count). A branch with its own fork (a fork inside a branch) is shown the same way.
4. **States of a row.** Each row in either scope has one of: *Applied* (on the main line), *Undone* (taken back and still redoable with Ctrl+Shift+Z or Ctrl+Shift+U), *On a branch* (taken back and not redoable linearly). Steps before a floor are wiped (`0041` criterion 30) and are not listed (criterion 7), so "wiped" is never a visible state. The state is also written as text, not colour alone: no word for Applied, "Undone" (or "Undone by <author>", `0020` criterion 39), and the semibold word "Branch" on the second line of a row on a branch. Test: one document through criteria 1 and 2 checks every row's state after each key press.
5. **Other peers.** Given a shared document, then in the Document scope a step by a peer appears on the main line in the order the document applied it and never starts a branch. A branch there holds only steps that someone took back. When a peer takes back their own step, the row says "Undone by Anna" and, if they then make a new step, that step's author has a branch in the same way. In the Object scope, following `0041` criterion 11, a new step on the object by any author ends the object's redo and puts the step taken back with object undo on a branch, while steps taken back with Ctrl+Z still stay on their own author's undo stack, so a peer never branches someone else's Ctrl+Z. Test with two replicas, in each scope.
6. **Saved.** Given Save and Open, then the branches are as before: ids, order, states. Nothing about branches is stored beyond the log (the architect derives them from the steps and the take-back marks of `0020`).
7. **Wipe.** Given a wipe (`0041` criteria 29 to 33 or `0020` criteria 47 to 51), then the wiped steps and their branches leave the scope's list; the rest follows those criteria.
8. **Rendering the lanes.** Given a list with branches, then the main line and at most 3 branches are drawn as lanes beside the rows (a vertical line with a mark at each step and a connector at the fork point). Each branch has a header row "Branch from a1b2c3d, 2 steps" directly above its newest row; a press or Enter on it folds the branch (its rows leave the list); branches start unfolded. Further branches (the fourth and later) fold into one row "N more branches" that unfolds in place into their headers, each folded. Header rows and the "N more branches" row are list items. A list with 50 branches and 10,000 steps opens within 200 ms.
9. **Keyboard.** Given the list, then Left and Right move focus into and out of a row's action button (Clone, or the objects disclosure; criteria 22 and 24), not between lanes; Enter on a header row folds it. A screen reader hears the lane information in the row's accessible name: "on branch from a1b2c3d, step 2 of 2". There is no lane-to-lane navigation; the list order is the order.

### Apply a version

10. **Apply** (changed, customer feedback 2026-10-14). Given a row in either scope, when the maker presses Enter on it or double-clicks it, then its version is applied as **one new step "Go to version <id>"** authored by me on my undo stack, and Ctrl+Z takes it back as a whole. In the Object scope the selected object's fields, tree parent and place in the stacking order become what they were at that version. In the Document scope the whole document does: every object changed since then gets its values back, an object created since then is removed, an object deleted since then comes back with its own id, and layers and groups get back their order and flags; document settings go back too, except the history wipe mark, which stays, as do clone marks and object wipe floors (ADR 0014 §8, amendment of 2026-10-14). If the object or the document already matches that version, nothing is written. Applying a version on the main line works the same as on a branch. The rows after that version keep their state, and a later new step forks (criterion 1). Every apply is its own step: three applies are three rows, and taking them all back takes three Ctrl+Z. An apply ends a preview that is showing, and the canvas shows the result. When the version lies far back or the apply writes many changes, the long-operation state of `0016` §9 shows first, as for undo (ADR 0014 §1). Test with `document.json` equality: in the Object scope, applying s1's version of an object now at s4 gives s1's state and Ctrl+Z gives s4's state again; in the Document scope, applying the version before a Draw and after a Delete removes the drawn object and brings back the deleted one with its id.
11. Withdrawn (customer feedback 2026-10-14). Was: exploring, where further presses joined one "Go to" step. Every apply is now its own step (criterion 10).
12. Withdrawn (customer feedback 2026-10-14). Was: the button **Restore previous state**. Ctrl+Z on an apply step does the same.
13. Withdrawn (customer feedback 2026-10-14). Was: leaving an exploration.
14. **Preview, then apply** (changed, customer feedback 2026-10-14). Given the History tab in either scope, then there is one way of working: no Mode row, no Preview/Live switch and no caption under it. A press or Space on a row starts a preview (criteria 17 and 19); Enter or a double-click applies (criterion 10); Escape ends a preview and changes nothing (criterion 18). A double-click on the step id selects the id (criterion 27) and does not apply. The panel shows no help text for this. The hint is only in each row's tooltip, "Click to preview. Double-click or Enter to apply this version.", and at the end of its accessible name, "Enter applies this version". Test: in both scopes the tab has no Mode row, no switch and no caption, and each row has that tooltip and accessible name.
15. **Whose steps.** Given an apply that sets fields which a later step of another peer wrote, then it writes them anyway (a deliberate return to an earlier state, unlike a take-back of one step), and the notice names the authors it overwrote: in the Object scope "Went to a1b2c3d. Anna's 2 later steps were reverted for this object.", in the Document scope "Went to a1b2c3d. Anna's 2 later steps were reverted." This follows `0041` Question 1; with its option B, applying over another peer's later step is refused. For the Document scope in a shared document this is open (customer question 2 below).
16. Given a drag, a chip, a long operation or an unfinished Pen path, then a press, Enter or double-click on a row is ignored, like `0020` criterion 6.

### Preview

17. **Object preview** (changed, customer feedback 2026-10-14). Given a press or Space on a row in the Object scope, then while the preview shows, the canvas draws the object only as it was at that version, in its place: the **blue preview outline** of the product's blue-and-black rule over its fill and stroke colours in a lighter copy (40 % opacity). The object's current drawing is hidden, and so are its selection box and handles; the version is not hoverable or selectable. The document does not change and no step is created. A readout says "Preview a1b2c3d, Resize, You, 12:03" (no control): it is the persistent line of the canvas notice slot and it is `role="status"`, so each newly pressed row is announced. When the preview ends without an apply, the current object, its selection box and its handles come back exactly as before. Test: during the preview the canvas holds the drawing of the version and none of the current object; after Escape `document.json` is unchanged and the current object is drawn again.
18. **Ending a preview.** Given a preview, then it ends, in the frame of the event, on: Escape in the list (the first Escape ends the preview, the second returns focus to the canvas), pressing the pinned "Now" row (`0020` criterion 39), which is also the newest state of the main line, selecting another object, switching tab, collapsing the panel, any press on the canvas (the press only ends the preview; it does nothing else), or an apply (criterion 10). Ending without an apply changes nothing. Pressing the row of the version the object already has ends it. In the Object scope an edit, undo or redo ends the preview first and then acts on the current object, which is drawn again.
19. **Document preview** (changed, customer feedback 2026-10-14). Given the Document scope and a press or Space on a row, then the canvas shows the whole document as it was right after that step, read-only: pointer input on the canvas and every editing key (undo and redo included) are ignored, except Enter or a double-click on a row of the list, which applies the version (criterion 10) and ends the preview; pan and zoom work, and three signs that do not depend on colour show that editing is paused: the tool rail is dimmed and inert, the pointer over the canvas is the plain arrow, and a 2 px inset frame in the preview colour surrounds the viewport (it takes no pointer events). A banner over the canvas says "Previewing a1b2c3d, 12:03. Editing is paused." with a button **Back to live** and, when other people's steps arrived meanwhile, "(3 new steps since)". Back to live, Escape in the list, or leaving the tab ends it. The live document keeps merging peers' updates underneath (the architect: the preview is read from the live document under a short checkout, plain copies kept, ADR 0014 §8).
20. **Preview is local.** Given a preview in either scope, then nothing is written, nothing is replicated, and no peer sees it. It is not an undoable thing and is not saved.
21. **Cost (ADR 0014 §13; release build, desktop; the tests assert four times each figure).** Given a document with 10,000 steps, then the first frame of a preview of a row up to 1,000 steps back from the current version appears within 300 ms, and moving through such rows with the Up and Down keys shows each within 300 ms. For a row further back the long-operation state of `0016` §9 shows before the read, and the first frame appears within 1.5 s at 10,000 steps back. A preview reads the objects it shows from the live document under a short checkout and keeps plain copies (criterion 32); the live document stays editable between rows. A preview is started by a press or Space on a row (Enter applies, criterion 10), never by focus alone: with no preview showing, Up and Down only move the focus (tabbing into the list and pressing Down never freezes the document); while a preview is showing, Up and Down move the preview along with the focus.

### Clone any state

22. **Clone.** Given a row of an object's timeline (any state, on a branch too, by any author), then the row has a button **Clone**, tooltip "Clone this version as a new object". Pressing it creates **a new object** with the geometry and style the object had at that version, placed at the same position on top of the stack and selected, as one step "Clone version a1b2c3d". The original does not change. Ctrl+Z removes the clone. Test: clone the version after s1 of an object that is now at s4; the clone equals s1's state.
23. **History of the clone.** Given the clone of criterion 22, then its timeline begins with "Clone of 3f9a1c at version a1b2c3d" and below it the inherited rows of the original **up to and including that version, along the path that led to it** (the main line or the branch, not the other branches), flagged "from 3f9a1c" (`0041` criteria 24 and 25). Later steps of the original never appear in it.
24. **Clone in the Document scope.** Given a row in the Document scope, then pressing its disclosure (a chevron button named "Objects of this step", `aria-expanded`) expands the row in place into one sub-row per touched object, at most 20, then a muted line "and N more"; one disclosure is open at a time (opening another closes the first). Each sub-row has a **Clone** button, including an object that step deleted ("Rectangle 3f9a1c, deleted"). A row whose step touched no object has no disclosure. For an object the step deleted, the clone has the state it had just before the deletion; for any other, the state right after the step. So a deleted object can be recovered as a clone even when nobody can undo the deletion any more. Test: draw, resize, delete, Save, Open, clone from the Delete row: a rectangle with the resized geometry.
25. **Groups.** Given a group's row, then no Clone button is shown in this version (the model of a group at a past version needs the subtree at that version, Question 5). A group's leaves can be cloned from their own timelines.
26. Given Clone, then it is not gated by a preview and not by the number of steps; it is gated by a drag, a chip, a long operation or an unfinished Pen path, like criterion 16.

### Step ids

27. Given a row, a notice or a tooltip that names a version, then it uses the step id of `0020` criterion 33 (7 characters or more, the same on every machine). Typing an id anywhere is not a feature. The id in a row is selectable text (a double-click selects it) so a maker can copy one into a message to a colleague; a tooltip cannot be selected because it closes with the pointer, so it is not the way to copy.

### What is derived and what is stored (ADR 0014, accepted; testable)

28. **Branches are derived from the log.** Given the one stack machine of `0020` criterion 55 run over one peer's steps (Document scope) or one object's timeline (Object scope), then a do step that arrives while the redo side holds steps leaves those steps as a branch; the fork point is the top of the undo side at that moment. Redo at a fork brings back the most recently left branch (criterion 2). Test: the same sequence fed live and read back from the log after Save and Open gives the same branches.
29. **Order of concurrent steps.** Given steps by two peers that were made concurrently, then every replica shows them in the same main-line order, by (Lamport timestamp, peer id), not by wall-clock time. The times in the rows may therefore look out of order by a few seconds; the same order on every replica matters more (ADR 0014 §6). Test: two replicas merge, the id sequence of the list is equal.
30. **Two exact versions per row.** Given a row, then "the version right after the step" is the document version at the step's last operation and "the version before the step" is the version its first change depended on; both are the same on every replica (ADR 0014, `adrs.md` decision 3). Test: preview and apply of one row give equal `document.json` fields on two replicas.
31. **Apply and clone version are steps with headers** (changed, customer feedback 2026-10-14). Given an apply or a clone of a version, then it is one commit whose header names the version and, for an apply in the Object scope and for a clone, the object; an apply in the Document scope names no object (ADR 0014 §2 and §8). Every apply is its own commit with its own header; no apply continues another.
32. **A preview is plain copies.** Given a preview and a step by a peer that arrives meanwhile, then the live document merges it and keeps editing underneath, the preview image does not change, and the count in the banner ("(3 new steps since)") grows by one. The preview is read from the live document under a short checkout (a guard that returns to the latest version, also when the read fails) and kept as plain copies of the objects, which cannot be edited; the live document stays detached only for the duration of one read, never between rows (ADR 0014 §8, §12).
33. **No new stored data.** Given branches, previews and applies, then none writes a key or a container beyond ordinary commits; only a clone of a version writes `clone_of` (`0041` criterion 37). `format_version` is unchanged (ADR 0014 Q2 A).

## Out of scope

- Merging two branches, naming or tagging branches or versions, comparing two versions (a diff view), cherry-picking one old step onto the current state.
- A Live mode, where a single press on a row changes the document, and a button that puts back the state from before a series of presses (withdrawn, customer feedback 2026-10-14). Ctrl+Z takes back each apply.
- An Apply button, and help text in the History tab about how to preview and apply (customer feedback 2026-10-14).
- A slider to scrub through versions with an animation, thumbnails per row, time-lapse export.
- Changing the number of lanes shown or sorting by author.
- Reverting an arbitrary older step of an object without going to its version (git's `revert` of a middle commit): not asked, and the field rule of `0020` criterion 16 makes it ambiguous.
- Restoring an object into the document with its original id when it is gone: a clone gets a new id (the original id belongs to the deleted object).
- Sharing a preview with a peer; a peer's cursor on another's preview.

## Notes for the architect (answered by ADR 0014 and `adrs.md`)

- A preview is read from the live document under a short checkout, plain copies kept (ADR 0014 §8 and §12, `adrs.md` decision 6); the document-wide preview is the set of copies. An apply, in either scope, is one run of the restore engine from the version to now (ADR 0014 §8, amendment of 2026-10-14). `revert_to` is not used, because it reverts every peer and re-creates deleted objects under new ids.
- Branches need no stored marks: the take-back marks are the undo and redo commits of `0020` criterion 56, and a concurrent step is placed by (Lamport, peer) (criterion 29). A peer's concurrent step is not a branch in the Document scope (criterion 5).
- Risks the architect measures: the first-frame cost of a checkout read against the budgets of criterion 21 (ADR 0014 §13), and checkout distance for old rows at 10,000 steps.

## Customer questions

Question 1 was raised by the architect's review of 2026-10-12 (`adrs.md`, Notes). **Decided: A** (customer, 2026-10-14); `0041` criterion 24 is reworded to match. Question 2 was raised by the architect with the customer feedback of 2026-10-14 and is open; it does not block the build, because sharing does not exist yet.

1. **Duplicate lineage.** Clone version and Duplicate now follow one lineage rule (criterion 23, ADR 0014): the inherited rows follow the path to the copied state. So the inherited rows of a Duplicate no longer show steps its author had taken back before the copy. This changes `0041` criterion 24, which the customer already accepted. *A (default, built):* a Duplicate shows only the steps in effect at the copy. *B:* a Duplicate keeps those steps, marked "Undone"; only Clone version follows the path. Recommendation A: one rule for both copies, and a copy's history lists only what made the copied shape. If unanswered, A stays and `0041` criterion 24 is reworded to match.
2. **Applying a document-wide version in a shared document (open; criteria 10 and 15).** An apply in the Document scope puts every object back, including objects other people changed after that version. *A (default, built):* the apply overwrites other people's later steps as one step that Ctrl+Z takes back, and the notice says how many it reverted ("Went to a1b2c3d. Anna's 2 later steps were reverted."). *B:* the apply is refused while someone else has later steps; the notice says whose. Recommendation A: it is what an apply means, nothing is lost because Ctrl+Z and the history keep the other steps, and the notice names them. If unanswered, A stays. Not blocking: sharing does not exist yet.

## Open questions (decided: the customer accepted every default, 2026-10-10)

Each question below is decided as its option A (or its stated default). The text stays as it was put.

1. **Redo at a fork (criterion 2).** After undo, undo, a new step, and then undoing the new step: *A (default, recommended):* Ctrl+Shift+Z brings back the newest branch (the new step); the older branch is reached through the list. *B:* Ctrl+Shift+Z does nothing while two branches hang from the fork point; the list highlights both and the maker picks one. *C:* time order: brings back the older branch first. The customer wrote "you can no longer redo linearly until you take back the conflict step"; A matches that.
2. **Live and preview (the customer is unsure).** *A (default, recommended):* the two-position switch of criterion 14, Preview by default. A click in the list never changes the document by accident, and in a shared document looking costs the others nothing; Live is one press of the switch away, and its Restore button is the "switch to restore the state from before the clicking" the customer asked for. *B:* Live only: a click always changes the object, the Restore button is always visible after the first click; fewer concepts, more accidents. *C:* Preview only in a preview pane inside the tab (a small picture of the version, no change on the canvas); safest, but you cannot see it at scale or in place. Recommendation A.
   **Changed by the customer feedback 2026-10-14:** Preview only, in both scopes. A press or Space previews; Enter or a double-click applies the version as one step; Escape leaves the preview. No switch, no Live mode, no Restore button (criteria 10 and 14; 11 to 13 withdrawn).
3. **Live in the Document scope (criterion 14).** *A (default):* not offered; the Document scope previews and clones. *B:* offered to go to a document-wide version, which reverts everybody's later steps for all objects; only for a document nobody else shares. Recommendation A.
   **Changed by the customer feedback 2026-10-14:** the Document scope applies a version with Enter or a double-click, as one step that Ctrl+Z takes back (criterion 10; ADR 0014 §8, amendment of 2026-10-14). It uses the restore engine, not `revert_to`, so deleted objects come back with their ids. What it does to other people's later steps in a shared document is customer question 2 above.
4. **Clone placement (criterion 22).** *A (default):* at the same position, on top, selected. *B:* offset by 10 mm so it is visible. The first keeps the state exact (the maker moves it); the second is easier to see. Recommendation A.
5. **Groups (criterion 25).** *A (default):* no Clone for a group version in this version. *B:* clone the group's whole subtree at that version. Larger; wanted when groups are common.
6. **Collapsing "Go to" steps (criterion 11).** *A (default):* consecutive go-to steps are one stack entry and one row. *B:* every press is its own step; the list fills during a browse and Restore needs several Ctrl+Z. Recommendation A.
   **Changed by the customer feedback 2026-10-14:** moot. Browsing is done with the preview, which writes nothing; every apply is its own step (criterion 10).

### The seven questions of ADR 0014 (decided 2026-10-10: option A each)

ADR 0014 is accepted; the customer took option A of every question below. The same seven questions stand in `0020` (Questions 11 to 17) and `0041`. For this spec Q6 decides criterion 15 (go to over another peer's later steps) and Q2 the file footprint of clone version.

7. **History in the file (ADR 0014 Q1).** *A (default):* keep it (it is there today), the History tab says so, a wipe is available. *B:* strip the history on every Save.
   **Changed by the customer feedback 2026-10-14:** the history stays in the file, but the History tab no longer says so (the line "The history is saved in the project file." is removed, `0020` criterion 45).
8. **File-format footprint (ADR 0014 Q2).** *A (default):* no `format_version` bump; step headers in commit messages and three optional keys (`history_wiped`, `history_floor`, `clone_of`) that older builds ignore. *B:* bump the version at `0041`, so older builds refuse files that carry lineage keys.
9. **Wipe in a shared document (ADR 0014 Q3).** *A (default):* the document wipe is not offered until sharing exists. *B:* admins only, with a new key epoch and a fresh sealed snapshot (ADR 0010 §8); copies other people already hold keep the old history. *C:* no wipe at all.
10. **Object wipe (ADR 0014 Q4).** *A (default):* it hides and cuts, the data stays in the file, the confirmation says so. *B:* no object wipe.
11. **Authors (ADR 0014 Q5).** *A (default):* a random id per session, shown as "You" and "Earlier session". *B:* one random id per installation in every step, so "You" survives reopening, but every file you share carries the same identifier. *C:* the OS user name in the file.
12. **Taking back someone else's step (ADR 0014 Q6; criterion 15).** With Ctrl+U or go to version, never with Ctrl+Z (ADR 0009 option C). *A (default):* allowed, as a named step that Ctrl+Z takes back. *B:* own steps only; go to version over another peer's later step is then refused.
13. **Undo of Delete (ADR 0014 Q7).** Decides whether a deleted object comes back with its own id, which a restore and a clone-from-deleted rely on. *A (default, recommended):* the object comes back with the **same id** through an injected Loro tree Move (ADR 0014 §11); no file-format change; Loro stays pinned with a canary test; soft delete is the fallback if an upgrade breaks the canary. *B:* soft delete now: deleted objects move into a hidden trash inside the file; public Loro API only, but a `format_version` bump (older builds refuse new files) and deleted objects stay in the file until a wipe. *C:* the object comes back under a **new id**: no risk, but selections, clone lineage and timelines break at every undone delete. Not recommended.

## UX notes

Written by the ux-engineer, 2026-10-10. Row anatomy, the pinned Now row, the canvas notice slot and the inline confirm are in `0020`; the scope group and the object rows are in `0041`. Numbers: `docs/design-system.md`, "Properties panel: History tab".

### Branches in a 244 px list

The list stays a flat list of rows. The branch is drawn in a gutter at the left of every row and named by a header row; no row changes height because of a branch.

| Part | Rule |
|---|---|
| Lanes | Lane pitch 12 px; main lane at x = 8 px, branch lanes at 20, 32, 44. Gutter width is 16 px + 12 px per extra lane the list needs, at most 3 branch lanes; a fourth and further branches fold into one row (below) |
| Main lane | 2 px `--toolbar-icon` line, the line the current state is on. It always sits in the leftmost lane, so "which branch is current" is "the left one" |
| Branch lane | 1.5 px `--lane-line` (`--toolbar-icon` at 60 %, 3.1:1), solid |
| Marks | Applied: filled 8 px disc. Undone (redoable, still on the main lane): 8 px ring on a dotted segment. On a branch: 8 px ring on a solid branch-lane line. Rings and discs differ by shape; position and the word differ the rest |
| Fork | At the fork-point row the main lane continues down and a connector leaves it to the right, a horizontal stub with a 4 px corner radius, then runs up the branch lane to the branch's newest row. No curves, no crossing lines: a branch that has a branch is drawn one lane further right, never inside another |
| Branch header row | 24 px, one per branch, directly above the branch's newest row: the lane line starts here; text "Branch from a1b2c3d, 2 steps" (12 px semibold `--panel-muted-fg`), a 12 px chevron at the right, `aria-expanded`. Press or Enter folds the branch (its rows leave the list), default unfolded |
| Beyond three | One row "3 more branches" (28 px, chevron) in place of the fourth and further headers; pressing unfolds in place into their headers, each folded |
| Status word | Line 2 of a row on a branch starts with the semibold word "Branch"; an undone row with "Undone"; applied rows have no word. So a state never rests on lane, ring or colour alone |
| Names | Accessible name adds "on branch from a1b2c3d, step 2 of 2" (criterion 9). The header row is the one place the branch is named in full |

The rows themselves, the 44 px height, the id and the author are those of `0020`. Drawing: each virtualised row draws its own slice of the gutter (an inline SVG, the lane segments precomputed from the log), so lines are continuous without a layer over the list.

### Preview and apply (changed, customer feedback 2026-10-14)

- **No Mode row, no caption slot, no Restore button.** The rows sit directly under the scope group and the Undo and Redo row, in both scopes. Nothing takes the freed space.
- **Row press or Space:** starts or moves the preview. The previewed row carries the selected look (`--value-fill` ground, 3 px bar) and `aria-current`.
- **Enter or double-click:** applies the version (criterion 10). The new row "Go to version a1b2c3d", glyph `Locate`, line 2 "Resize · You · 12:06", appears at the top of the list; the notice of the table below shows. A double-click on the id text selects the id instead (criterion 27).
- **Hint:** only in the row's tooltip, "Click to preview. Double-click or Enter to apply this version.", and at the end of the row's accessible name, "Enter applies this version" (criterion 14). No visible help text in the panel.

### How a preview is drawn

A preview is never a change, so it must not look like one.

| Case | Look |
|---|---|
| Object preview | (Changed, customer feedback 2026-10-14.) The current object is not drawn while the preview lasts, and neither are its selection box and handles. Only the version is drawn, in its place: a hollow 1.5 px `--preview-new` outline with the white casing, plus a copy of its fill and stroke at 40 % (`--preview-ghost`) underneath the outline. The outline is what applying the version would produce, the same meaning as blue during a drag. The version is not hoverable or selectable. When the preview ends, the current object and its handles are drawn again in the same frame |
| Readout | The persistent line of the canvas notice slot, no controls: "Preview a1b2c3d, Resize, You, 12:03". `role="status"`, so each new row pressed is announced |
| Document preview | The canvas shows the document as it was after that step, plain, no blue on the objects. Three signs that do not depend on colour: the banner in the slot "Previewing a1b2c3d, 12:03. Editing is paused." with the button **Back to live** and the muted "(3 new steps since)"; the tool rail dimmed and inert; the pointer is the plain arrow over the canvas. Plus a 2 px inset `--preview-new` frame around the viewport (`pointer-events: none`) as the first thing the eye sees. Pan and zoom work |
| Ending | Criterion 18 and 19. The first Escape in the list ends the preview, the second returns focus to the canvas. Ending happens in the frame of the event; no fade |

### Clone and the touched-objects disclosure

- **Object scope row:** a 24 × 24 icon button `CopyPlus` in the row-action slot, tooltip "Clone this version as a new object" (criterion 22). After it: the clone is selected, the notice "Cloned version a1b2c3d as a new object." and, because the selection is now exactly one object, the History tab shows the clone's timeline.
- **Document scope row:** the row action is a chevron button named "Objects of this step", `aria-expanded`. Pressing it expands the row **in place** by one 28 px sub-row per touched object, indented under the kind glyph: glyph of the kind, "Rectangle 3f9a1c", the muted word "deleted" where it is, and a `CopyPlus` button at the right. At most 20 sub-rows, then the muted line "and 12 more"; one disclosure open at a time (opening another closes it), so the virtualiser keeps a simple height model. A group's sub-row has no Clone button (Question 5 A). A row whose step touched no object (Resize document) has no row action.
- **Gate:** all of it follows criterion 26.

### Keyboard

Up, Down, Home, End, Page Up, Page Down move between rows, header rows included. Space on a row starts a preview; Enter applies its version (changed, customer feedback 2026-10-14). While a preview is showing, Up and Down move the preview along with the focus, within 300 ms (criterion 21); with none showing they only move the focus, so tabbing into the list and pressing Down never freezes the document. Right moves into the row's action button, Left back; Enter on a header row folds it. Escape: first ends a preview, second returns focus to the canvas. There is no lane-to-lane navigation: lanes are drawing, the list order is the order.

### Notices (canvas notice slot)

| Event | Text |
|---|---|
| Apply | "Went to a1b2c3d." |
| Apply over another author's later steps, Object scope | "Went to a1b2c3d. Anna's 2 later steps were reverted for this object." |
| Apply over another author's later steps, Document scope | "Went to a1b2c3d. Anna's 2 later steps were reverted." |
| Clone | "Cloned version a1b2c3d as a new object." |

### Criteria changes requested and applied

All changes the ux-engineer requested (criteria 4, 8, 9, 12, 14, 17, 18, 19, 21, 24, 27) were applied to the criteria above by the product owner on 2026-10-10. The customer feedback of 2026-10-14 removed the Mode row, the caption slot, the Restore button and the exploration row; the product owner updated these notes to match. The ux-engineer reviews them with the next UI pass.

### Design questions: decided (2026-10-10)

1. A press on a row in the Document scope freezes editing: it starts a read-only preview with a banner (criterion 19). Decided as specified, with Back to live and Escape always one step away.
2. Lane drawing: decided as a per-row slice of the gutter (an inline SVG per virtualised row), not a graph layer over the list, because it virtualises.

## Links

Requirements: R-HIST-003
Depends on: `specs/0020-undo-redo/`, `specs/0041-object-history/`, `specs/0043-properties-tabs/`
Related: ADR 0009 §1 (option C), ADR 0002 §9, the blue-and-black preview rule of `specs/0009-unified-object-editing/`
ADRs: `adrs.md` (architect); ADR 0014 (accepted 2026-10-10: how branches and previews are derived; §8 amended 2026-10-14: applying a version)
PR: #119
