# Sketcher UI study

Status: visual proposal, 2026-10-30. The customer requested a preview before
production implementation. This is a standalone HTML/SVG study, not application
code and not evidence that the solver or sketch model works.

Open `index.html` locally in a browser; no server, packages or network are needed.

- Use the Vector/Sketch workbench selector above the existing Properties dock.
- Select tools in the floating left rail to inspect construction methods in
  the single floating top options bar.
- Switch between Style and Sketches in the vertical property tab strip.
  Constraints and parameters share the Sketches tab.
- Use the illustrative-state controls inside the Sketches property pane to inspect
  solver-status presentation. There is no bottom demo toolbar; these controls
  belong only to this static study, not the production UI.
- In Conflict, use **Remove added dimension** to return to the valid sample.
- Switch to Vector and back to Sketch to inspect the proposed workbench navigation.
- Width has a limited scripted preview; it does not solve a constraint system.
  Dimension extension lines and the rest of the diagram are illustrative.

`sketcher.png` shows Sketch, `sketcher-conflict.png` shows diagnosis, and
`vector.png` shows the Vector workbench with Style properties.
Colours reuse the current application palette. Placement, control grouping,
icons and navigation remain reviewable proposals; no new design-system rules
or global shortcuts are established by this study.

Validation: opened with headless Chromium at 1680 × 1000. Checked workbench/tool
switching, vertical Style/Sketches tabs, construction variants, conflict diagnosis
and repair, no JavaScript errors and no horizontal overflow. No application or
solver tests run. Other property tabs are visual context only; Inspect updates
illustrative selection text rather than computing/highlighting degrees of freedom.
