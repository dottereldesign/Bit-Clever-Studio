import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Channel, invoke } from "./diagnostics";
import { LoaderCircle, RotateCcw, X } from "lucide-react";
import type { Library } from "./model";
type Scan = { groups: number[][]; skipped: number };
type Progress = {
  phase: "waiting" | "indexing" | "checking" | "grouping";
  checked: number;
  total: number;
  current: string;
  elapsed_ms: number;
  remaining_seconds: number | null;
};

function duration(seconds: number) {
  const value = Math.max(0, Math.ceil(seconds));
  if (value < 60) return `${value}s`;
  if (value < 3600) return `${Math.floor(value / 60)}m ${value % 60}s`;
  return `${Math.floor(value / 3600)}h ${Math.floor((value % 3600) / 60)}m`;
}
type Recovery = { id: number; name: string; path: string };
export default function DuplicatesDialog({
  lib,
  close,
  refresh,
}: {
  lib: Library;
  close: () => void;
  refresh: () => Promise<unknown>;
}) {
  const [scan, setScan] = useState<Scan | null>(null),
    [progress, setProgress] = useState<Progress | null>(null),
    [checking, setChecking] = useState(true),
    [elapsed, setElapsed] = useState(0),
    [recovery, setRecovery] = useState<Recovery[]>([]),
    [keepers, setKeepers] = useState<Record<string, number>>({}),
    [busy, setBusy] = useState(true),
    [error, setError] = useState("");
  const mounted = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  useLayoutEffect(() => {
    const element = dialog.current!;
    element.showModal();
    return () => {
      element.close();
      requestAnimationFrame(() => {
        if (!element.isConnected) {
          const target = previousFocus.current?.isConnected ? previousFocus.current : document.querySelector<HTMLElement>('[aria-label="Settings"]');
          target?.focus({preventScroll:true});
        }
      });
    };
  }, []);
  useEffect(() => {
    const element=dialog.current;
    if(busy&&!checking)element?.focus();
    else if(document.activeElement===element)element?.querySelector<HTMLButtonElement>('[aria-label="Close duplicates"]')?.focus();
  },[busy,checking]);
  const reload = async () => {
    setChecking(true);
    setProgress(null);
    setScan(null);
    const onProgress = new Channel<Progress>();
    onProgress.onmessage = (value) => {
      if (mounted.current) setProgress(value);
    };
    try {
      const [found, removed] = await Promise.all([
        invoke<Scan>("find_duplicates", { onProgress }),
        invoke<Recovery[]>("duplicate_recovery"),
      ]);
      if (mounted.current) {
        setScan(found);
        setRecovery(removed);
      }
    } finally {
      if (mounted.current) setChecking(false);
    }
  };
  useEffect(() => {
    if (!busy) return;
    const start = Date.now();
    setElapsed(0);
    const timer = window.setInterval(() => setElapsed((Date.now() - start) / 1000), 1000);
    return () => window.clearInterval(timer);
  }, [busy]);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    // Strict Mode's first effect is cleaned up before this microtask runs.
    Promise.resolve().then(() => active ? reload() : undefined)
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
      mounted.current = false;
    };
  }, []);
  const act = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setChecking(false);
    setError("");
    try {
      await action();
      await refresh();
      await reload();
    } catch (e) {
      setError(String(e));
      await refresh();
      await reload().catch(() => {});
    } finally {
      setBusy(false);
    }
  };
  return (
    <dialog
      ref={dialog}
      className="modal-backdrop duplicates-dialog"
      aria-label="Duplicates"
      tabIndex={-1}
      onCancel={event=>{event.preventDefault();if(!busy||checking)close();}}
      onKeyDown={event=>{
        event.stopPropagation();
        if(event.key!=='Tab')return;
        const controls=Array.from(event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled)')).filter(element=>element.tabIndex>=0&&element.getClientRects().length>0&&(!(element instanceof HTMLInputElement)||element.type!=='radio'||element.checked));
        const first=controls[0],last=controls.at(-1),active=document.activeElement;
        if(!first){event.preventDefault();event.currentTarget.focus();}
        else if(event.shiftKey&&(active===first||active===event.currentTarget)){event.preventDefault();last?.focus();}
        else if(!event.shiftKey&&(active===last||active===event.currentTarget)){event.preventDefault();first.focus();}
      }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget && (!busy || checking)) close();
      }}
    >
      <div
        className="modal duplicates-modal"
      >
        <div className="modal-heading">
          <h2>Duplicates</h2>
          <button
            className="icon-button"
            aria-label="Close duplicates"
            autoFocus
            disabled={busy&&!checking}
            onClick={close}
          >
            <X size={17} />
          </button>
        </div>
        {busy && (
          <div className="duplicate-loading">
            <div className="duplicate-progress-heading" role="status">
              <span><LoaderCircle size={14} className="spin" />
                {!checking ? "Updating files…" : progress?.phase === "indexing" ? "Indexing library…" : progress?.phase === "grouping" ? "Comparing matches…" : progress?.phase === "checking" ? "Checking file contents…" : "Waiting for library…"}
              </span>
              <span>{checking && progress && progress.phase !== "waiting" ? `${Math.floor(progress.checked / Math.max(1, progress.total) * 100)}%` : ""}</span>
            </div>
            <progress
              className="duplicate-progress-bar"
              aria-label="Duplicate check progress"
              max={Math.max(1, progress?.total ?? 0)}
              value={checking && progress && progress.phase !== "waiting" ? (progress.total ? progress.checked : 1) : undefined}
            />
            <div className="duplicate-progress-details">
              <span>{checking && progress && progress.phase !== "waiting" ? `${progress.checked.toLocaleString()} / ${progress.total.toLocaleString()} files` : ""}</span>
              <span>{duration(elapsed)} elapsed{checking && progress?.remaining_seconds != null ? ` · ~${duration(progress.remaining_seconds)} left` : checking && (progress?.phase === "checking" || progress?.phase === "indexing") ? " · Estimating…" : ""}</span>
            </div>
            {checking && progress?.current && <div className="duplicate-progress-file" title={progress.current}>{progress.current}</div>}
          </div>
        )}
        {scan && !scan.groups.length && (
          <div className="modal-empty" role="status">No duplicates found</div>
        )}
        {scan && !busy && scan.groups.length>0 && <p role="status">{scan.groups.length} duplicate {scan.groups.length===1?'group':'groups'} found. Choose which file to keep in each group.</p>}
        {scan?.groups.map((ids, groupIndex) => {
          const key = ids.join("-"),
            keeper = keepers[key] ?? ids[0];
          return (
            <fieldset className="duplicate-group" key={key}><legend>Group {groupIndex+1}: choose the file to keep</legend>
              {ids.map((id) => {
                const item = lib.items.find((i) => i.id === id);
                return (
                  <label key={id} className="duplicate-file">
                    <input
                      type="radio"
                      disabled={busy}
                      name={`keep-${key}`}
                      checked={keeper === id}
                      onChange={() => setKeepers({ ...keepers, [key]: id })}
                    />
                    <span>
                      <strong>{item?.name || `File ${id}`}</strong>
                      <small title={item?.path}>{item?.path}</small>
                    </span>
                    <small aria-hidden="true">{keeper === id ? "Keep" : ""}</small>
                  </label>
                );
              })}
              <button
                className="subtle-button"
                disabled={busy}
                onClick={() =>
                  void act(() =>
                    invoke("remove_duplicates", {
                      keeper,
                      ids: ids.filter((id) => id !== keeper),
                    }),
                  )
                }
              >
                Keep selected · remove {ids.length - 1}{" "}
                {ids.length === 2 ? "copy" : "copies"}
              </button>
            </fieldset>
          );
        })}
        {!!scan?.skipped && (
          <small>{scan.skipped} unavailable files skipped</small>
        )}
        {scan && (
          <p className="duplicate-note">
            Tags, projects and pins merge into the retained copy. Removed files
            stay in recovery.
          </p>
        )}
        {recovery.length > 0 && (
          <div className="recovery-list">
            <h3>Recovery</h3>
            {recovery.map((file) => (
              <div key={file.id}>
                <span title={file.path}>{file.name}</span>
                <button
                  disabled={busy}
                  aria-label={`Restore ${file.name}`}
                  onClick={() =>
                    void act(() => invoke("restore_duplicate", { id: file.id }))
                  }
                >
                  <RotateCcw size={12} /> Restore
                </button>
              </div>
            ))}
          </div>
        )}
        {error && (
          <div className="form-error" role="alert">
            {error}
          </div>
        )}
      </div>
    </dialog>
  );
}
