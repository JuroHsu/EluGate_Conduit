/** Subscribe before reading the snapshot; a late reply must not undo a newer event. */
export function syncLanguage(
  read: () => Promise<string>,
  subscribe: (onChange: (lang: string) => void) => Promise<() => void>,
  apply: (lang: string) => void,
) {
  let disposed = false;
  let revision = 0;
  let unlisten: (() => void) | undefined;
  const onChange = (lang: string) => {
    revision++;
    if (!disposed) apply(lang);
  };
  void (async () => {
    try {
      unlisten = await subscribe(onChange);
      if (disposed) {
        unlisten();
        return;
      }
      const before = revision;
      const lang = await read();
      if (!disposed && before === revision) apply(lang);
    } catch {
      if (!disposed && revision === 0) apply("en");
    }
  })();
  return () => {
    disposed = true;
    unlisten?.();
  };
}
