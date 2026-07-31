export function markWindowDocument(
  windowLabel: string,
  root: HTMLElement = document.documentElement,
) {
  root.dataset.window = windowLabel;
}
