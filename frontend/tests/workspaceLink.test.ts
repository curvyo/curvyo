import assert from "node:assert/strict";
import { test } from "node:test";

import { moveWorkspaceLink, onWorkspaceLinkMoved, type LinkHolder } from "../src/lib/workspaceLink.ts";

class Holder implements LinkHolder {
  link: object | null;
  constructor(link: object | null) {
    this.link = link;
  }
  has_workspace_link() {
    return this.link !== null;
  }
  take_workspace_link() {
    const link = this.link;
    this.link = null;
    return link ?? undefined;
  }
  attach_workspace_link(link: never) {
    this.link = link;
  }
}

test("the workspace link moves to the next session and the transport hears of it", () => {
  const link = { id: 1 };
  const from = new Holder(link);
  const to = new Holder(null);
  const moved: LinkHolder[] = [];
  const stop = onWorkspaceLinkMoved((next) => moved.push(next));
  assert.equal(moveWorkspaceLink(from, to), true);
  assert.equal(to.link, link);
  assert.equal(from.link, null);
  assert.deepEqual(moved, [to]);
  stop();
  assert.equal(moveWorkspaceLink(new Holder(null), to), false, "nothing to move");
  assert.equal(moveWorkspaceLink(null, to), false);
});
