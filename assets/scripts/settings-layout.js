(() => {
  const GRID_SELECTOR = ".settings-card-grid";
  const observedGrids = new Set();
  const observedItems = new Set();
  const pendingGrids = new Set();
  let animationFrame = 0;

  function scheduleLayout(grid) {
    if (!(grid instanceof HTMLElement) || !grid.isConnected) {
      return;
    }

    pendingGrids.add(grid);
    if (animationFrame !== 0) {
      return;
    }

    animationFrame = requestAnimationFrame(() => {
      animationFrame = 0;
      const grids = Array.from(pendingGrids);
      pendingGrids.clear();
      grids.forEach(layoutGrid);
    });
  }

  function layoutGrid(grid) {
    const style = getComputedStyle(grid);
    const rowHeight = Number.parseFloat(style.gridAutoRows);
    const cardGap = Number.parseFloat(style.getPropertyValue("--masonry-card-gap"));
    if (!Number.isFinite(rowHeight) || !Number.isFinite(cardGap)) {
      return;
    }

    for (const item of grid.children) {
      if (!(item instanceof HTMLElement)) {
        continue;
      }

      const height = item.getBoundingClientRect().height;
      const span = Math.max(1, Math.ceil((height + cardGap) / rowHeight));
      const nextValue = `span ${span}`;
      if (item.style.gridRowEnd !== nextValue) {
        item.style.gridRowEnd = nextValue;
      }
    }
  }

  function observeItem(item) {
    if (!(item instanceof HTMLElement) || observedItems.has(item)) {
      return;
    }

    observedItems.add(item);
    resizeObserver.observe(item);
  }

  function observeGrid(grid) {
    if (!(grid instanceof HTMLElement)) {
      return;
    }

    if (!observedGrids.has(grid)) {
      observedGrids.add(grid);
      resizeObserver.observe(grid);
    }

    for (const item of grid.children) {
      observeItem(item);
    }
    layoutGrid(grid);
  }

  function removeDetachedObservations() {
    for (const grid of observedGrids) {
      if (!grid.isConnected) {
        resizeObserver.unobserve(grid);
        observedGrids.delete(grid);
        pendingGrids.delete(grid);
      }
    }

    for (const item of observedItems) {
      if (!item.isConnected) {
        resizeObserver.unobserve(item);
        observedItems.delete(item);
      }
    }
  }

  function reconcileLayout() {
    removeDetachedObservations();
    document.querySelectorAll(GRID_SELECTOR).forEach(observeGrid);
  }

  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const element = entry.target;
      const grid = element.matches(GRID_SELECTOR) ? element : element.parentElement;
      if (grid?.matches(GRID_SELECTOR)) {
        scheduleLayout(grid);
      }
    }
  });

  const mutationObserver = new MutationObserver(reconcileLayout);

  function start() {
    reconcileLayout();
    mutationObserver.observe(document.body, { childList: true, subtree: true });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start, { once: true });
  } else {
    start();
  }
})();
