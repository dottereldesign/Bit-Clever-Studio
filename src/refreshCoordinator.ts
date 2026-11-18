// One read at a time. An event arriving during a read requires a fresh read:
// neither the obsolete snapshot nor its error may reach UI/history consumers.
// Every caller waits for the same final snapshot, including save receipts.
export function createRefreshCoordinator<T>(load: () => Promise<T>, apply: (value: T) => void) {
  let revision = 0;
  let pending: Promise<T> | undefined;
  return function refresh(): Promise<T> {
    revision++;
    if (!pending) {
      pending = Promise.resolve().then(async () => {
        try {
          for (;;) {
            const requested = revision;
            let value: T;
            try {
              value = await load();
            } catch (error) {
              if (requested !== revision) continue;
              throw error;
            }
            if (requested !== revision) continue;
            apply(value);
            if (requested !== revision) continue;
            return value;
          }
        } finally {
          pending = undefined;
        }
      });
    }
    return pending;
  };
}
