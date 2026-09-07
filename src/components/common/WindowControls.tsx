import { useEffect, useState } from "react";
import { Minus, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function WindowControls({ dark = true }: { dark?: boolean }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let active = true;
    const sync = async () => {
      try {
        const value = await getCurrentWindow().isMaximized();
        if (active) setMaximized(value);
      } catch { /* browser preview / unavailable window API */ }
    };
    void sync();
    return () => { active = false; };
  }, []);

  const minimize = async () => { try { await getCurrentWindow().minimize(); } catch {} };
  const toggleMaximize = async () => {
    try {
      const win = getCurrentWindow();
      const next = !await win.isMaximized();
      await win.toggleMaximize();
      setMaximized(next);
    } catch {}
  };
  const close = async () => { try { await getCurrentWindow().close(); } catch {} };

  const base = dark ? "text-[#7f857c] hover:bg-[#222522] hover:text-white" : "text-[#6d726c] hover:bg-[#e8ebe7] hover:text-[#111]";
  return (
    <div className="window-controls" data-tauri-drag-region="false">
      <button type="button" aria-label="Minimize" title="Minimize" onClick={() => void minimize()} className={`window-control ${base}`}><Minus className="h-3.5 w-3.5" /></button>
      <button type="button" aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onClick={() => void toggleMaximize()} className={`window-control ${base}`}>
        <Square className="h-3 w-3" />
      </button>
      <button type="button" aria-label="Close" title="Close" onClick={() => void close()} className={`window-control window-control-close ${base}`}><X className="h-3.5 w-3.5" /></button>
    </div>
  );
}
