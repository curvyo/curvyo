# Parameter handle clone: Ctrl on a parameter handle makes a copy instead of changing the shape

Status: Draft (a sketch for the customer; builds on `0021` and its parameter handles)
Priority: Should
Origin: Customer (2026-10-19, translated): "If one presses Ctrl on the inner radius or the curve handle, a clone is made instead of a modification. Likewise a checkbox when one double-clicks on it." Decided by the customer on the same day: the copy is an ordinary copy like Duplicate (CURVE-CLONE-KIND A), it applies to all parameter handles (CURVE-CLONE-SCOPE A), and it gets no offset.

## User value

As a maker I want to pull a handle and get a new shape with the new value while the original stays as it is, so that a flower, a sun or a row of stars with different curves is made from one shape in a few drags, without Duplicate and Undo in between.

## Terms

- **Parameter handle**: a handle of the Select tool that changes one parameter of a primitive: Curve, inner radius (star), Start and End (arc), corner radius and number of sides (`0021`, `0009`, `0052`).
- **Copy**: an ordinary new object, as Duplicate makes it: new id, same style, same layer and z-order position directly above the original, independent of it afterwards. Not a linked clone (`0049`).

## Acceptance criteria (sketch, to be sharpened before Ready)

1. Given a parameter handle is dragged with Ctrl held at the press, then the original is unchanged during and after the drag, and one new object is made that has the dragged parameter value; Ctrl is read live as in `0014`, so releasing Ctrl before the release of the button changes the original instead (the copy is only made if Ctrl is held at the end of the drag; the live preview shows the copy's outline as the blue/black preview of the existing parameter drag).
2. Given the copy is made, then it lies at **exactly the same position** as the original, with **no offset**, directly above it in z-order, and it is selected (the original is not).
3. Given the entry box of a parameter handle is open (double-click, `0021`), then it has a check "As copy" (a check box inside the box, not a button, not a panel section); with the check set, Enter applies the typed value to a copy exactly as in criteria 1 and 2; Escape closes the box; the check starts unset every time.
4. Given criterion 1 or 3, then the copy is one commit with its own label ("Copy with curve" and so on) and one undo step removes the copy and leaves the original (`0020`); the object history of the copy begins "Duplicate of ..." (`0041`).
5. Given a value that the shape limits (`0021`, "limited"), then the copy gets the limited value and the same short notice.
6. Given the dragged handle belongs to a selection of several shapes, then Ctrl copies all of them (each copy gets the value its own shape gets, as in the multi-selection drag of `0021`).
7. Ctrl on a node handle or on a size handle keeps its meaning from `0010` and `0005`; this story changes only parameter handles.
8. There is no hint text, no tip and no new panel section (`docs/design-system.md`, Interaction conventions). The check in the box is the only new element.

## Out of scope

- A linked clone (`0049`): the customer chose an ordinary copy.
- An offset or a placement dialog for the copy.
- Ctrl on the Select bar fields (the typed value goes through the box with its check only).

## Open points for the sketch

- The key: Ctrl is the customer's wish; it is also the "square" and the "snap" modifier on other drags, so the sharpening checks the key table for a conflict while a parameter handle is dragged (the snap of `0021` criterion 56 uses Ctrl).
