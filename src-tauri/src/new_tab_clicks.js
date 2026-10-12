// Opens links in a new tab on ⌘-click (Ctrl-click on Linux and Windows) and on middle-click, as in Chrome.
// The tab opens in the background. With Shift, it opens in the foreground.
// The app reads the "jira-background-tab:" prefix in on_new_window.
(() => {
  const isMac = navigator.platform.startsWith("Mac");

  const onClick = (event) => {
    const newTabKey = isMac ? event.metaKey : event.ctrlKey;
    const newTabClick = event.button === 1 || (event.button === 0 && newTabKey);
    if (!newTabClick) return;

    const link = event.composedPath().find((node) => node instanceof HTMLAnchorElement && node.href);
    if (!link || !["http:", "https:"].includes(link.protocol)) return;

    event.preventDefault();
    event.stopImmediatePropagation();
    window.open(event.shiftKey ? link.href : `jira-background-tab:${link.href}`);
  };

  // Capture on window, so that the page's own link handlers do not run first
  window.addEventListener("click", onClick, true);
  window.addEventListener("auxclick", onClick, true);
})();
