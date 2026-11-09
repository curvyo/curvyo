// Records every selection of the Properties panel's tab strip (`specs/0105` criterion 20): a
// MutationObserver installed before an action notes the selected tab on every DOM change, so a
// flash of another tab for one frame is caught where polling could miss it.

/** Starts recording on `page`; read with `tabsSeen`. */
async function watchTabs(page) {
  await page.evaluate(() => {
    const read = () => {
      const strip = document.querySelector('[role=tablist][aria-label="Panel"]');
      const tab = strip?.querySelector('[role=tab][aria-selected=true]');
      return tab ? tab.getAttribute('aria-label') || '?' : 'none';
    };
    window.__tabsObserver?.disconnect();
    window.__tabsSeen = [read()];
    window.__tabsObserver = new MutationObserver(() => {
      const now = read();
      if (window.__tabsSeen[window.__tabsSeen.length - 1] !== now) window.__tabsSeen.push(now);
    });
    window.__tabsObserver.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ['aria-selected', 'data-state'],
    });
  });
}

/** The tabs selected since `watchTabs`, each change once, and stops recording. */
async function tabsSeen(page) {
  return page.evaluate(() => {
    window.__tabsObserver?.disconnect();
    return window.__tabsSeen;
  });
}

module.exports = { watchTabs, tabsSeen };
