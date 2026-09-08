import { useEffect, useState } from "react";
import { Minus, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function WindowControls({ dark = true }: { dark?: boolean }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    try {
      const win = getCurrentWindow();
      const sync = async () => {
        try { const value = await win.isMaximized(); if (active) setMaximized(value); } catch {}
      };
      void sync();
      void win.onResized(() => { void sync(); }).then(fn => { unlisten = fn; });
    } catch {}
    return () => { active = false; unlisten?.(); };
  }, []);

  const run = async (action: () => Promise<void>) => {
    try { await action(); } catch (error) { console.error("Window control failed", error); }
  };
  const minimize = () => run(() => getCurrentWindow().minimize());
  const toggleMaximize = () => run(async () => {
    const win = getCurrentWindow();
    await win.toggleMaximize();
    setMaximized(await win.isMaximized());
  });
  const close = () => run(() => getCurrentWindow().close());

  const base = dark ? "text-[#7f857c] hover:bg-[#222522] hover:text-white" : "text-[#6d726c] hover:bg-[#e8ebe7] hover:text-[#111]";
  return (
    <div className="window-controls" role="group" aria-label="Window controls">
      <button type="button" aria-label="Minimize" title="Minimize" onPointerDown={e => e.stopPropagation()} onClick={() => void minimize()} className={`window-control ${base}`}><Minus className="h-3.5 w-3.5" /></button>
      <button type="button" aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onPointerDown={e => e.stopPropagation()} onClick={() => void toggleMaximize()} className={`window-control ${base}`}>
        <Square className="h-3 w-3" />
      </button>
      <button type="button" aria-label="Close" title="Close" onPointerDown={e => e.stopPropagation()} onClick={() => void close()} className={`window-control window-control-close ${base}`}><X className="h-3.5 w-3.5" /></button>
    </div>
  );
}
