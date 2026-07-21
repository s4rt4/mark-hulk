// Proportional scroll sync between the editor and preview panes (Split view).
// Each pane registers its scroll element; when one scrolls, the other follows
// at the same relative position. A short-lived lock prevents feedback loops
// (setting scrollTop programmatically fires a scroll event on the other pane).

const panes = { editor: null, preview: null };
let lock = null;
let lockTimer = null;

/** Register a pane's scroll element. Returns an unregister function. */
export function registerPane(name, el) {
  panes[name] = el;
  return () => {
    if (panes[name] === el) panes[name] = null;
  };
}

/** Mirror `name`'s scroll position onto the other pane. */
export function syncFrom(name) {
  if (lock && lock !== name) return;
  const src = panes[name];
  const dst = panes[name === "editor" ? "preview" : "editor"];
  if (!src || !dst) return;

  lock = name;
  clearTimeout(lockTimer);
  lockTimer = setTimeout(() => (lock = null), 120);

  const srcMax = src.scrollHeight - src.clientHeight;
  const dstMax = dst.scrollHeight - dst.clientHeight;
  if (srcMax <= 0 || dstMax <= 0) return;
  dst.scrollTop = (src.scrollTop / srcMax) * dstMax;
}
