import { useEffect, useState } from "react";
import { CheckCircle2, Loader2, Printer, RefreshCw, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { reportsService, type PrinterInfo } from "@/services/reports";

export type PrintOrientation = "portrait" | "landscape";

export interface PrintSettings {
  printer: string | null;
  paper: string;
  orientation: PrintOrientation;
  copies: number;
}

interface Props {
  title: string;
  initialOrientation?: "portrait" | "landscape";
  onClose: () => void;
  onSavePdf: (settings: PrintSettings) => Promise<void>;
  onPrint: (settings: PrintSettings) => Promise<void>;
}

export function PrintConfigModal({ title, initialOrientation = "portrait", onClose, onSavePdf, onPrint }: Props) {
  const [printers, setPrinters] = useState<PrinterInfo[]>([]);
  const [printer, setPrinter] = useState<string | null>(null);
  const [paper, setPaper] = useState("A4");
  const [orientation, setOrientation] = useState(initialOrientation);
  const [copies, setCopies] = useState(1);
  const [busy, setBusy] = useState<"print" | "pdf" | "load" | null>("load");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  const loadPrinters = async () => {
    setBusy("load"); setError("");
    try {
      const rows = await reportsService.getPrinters();
      setPrinters(rows);
      const preferred = rows.find(p => p.is_default) ?? rows[0];
      setPrinter(preferred?.name ?? null);
      if (!preferred) setError("No printer was detected. Add/configure a system printer before printing.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to load printers.");
    } finally { setBusy(null); }
  };

  useEffect(() => { void loadPrinters(); }, []);

  const settings: PrintSettings = { printer, paper, orientation, copies: Math.max(1, Math.min(99, copies || 1)) };

  const savePdf = async () => {
    setBusy("pdf"); setError(""); setMessage("");
    try { await onSavePdf(settings); setMessage("PDF created successfully in the configured report output folder."); }
    catch (e) { setError(e instanceof Error ? e.message : "Unable to create PDF."); }
    finally { setBusy(null); }
  };

  const print = async () => {
    setBusy("print"); setError(""); setMessage("");
    try { await onPrint(settings); setMessage(`Print job sent to ${printer}.`); }
    catch (e) { setError(e instanceof Error ? e.message : "Unable to print."); }
    finally { setBusy(null); }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm">
      <Card className="w-full max-w-xl border-[#343434] bg-[#121212] p-0 shadow-2xl">
        <div className="flex items-start justify-between border-b border-[#292929] px-5 py-4">
          <div>
            <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#777]">Printing configuration</p>
            <h3 className="mt-1 text-base font-semibold text-white">{title}</h3>
          </div>
          <button type="button" onClick={onClose} className="rounded-lg p-2 text-[#777] hover:bg-[#202020] hover:text-white" aria-label="Close"><X className="h-4 w-4" /></button>
        </div>

        <div className="space-y-4 p-5">
          {error && <div className="rounded-xl border border-[#59462b] bg-[#1b1710] px-3 py-2.5 text-xs text-[#e7d0a4]">{error}</div>}
          {message && <div className="flex items-center gap-2 rounded-xl border border-[#294126] bg-[#142012] px-3 py-2.5 text-xs text-[#b9dfb2]"><CheckCircle2 className="h-4 w-4" />{message}</div>}

          <div>
            <label className="text-xs font-medium text-[#aaa]">Printer</label>
            <div className="mt-1 flex gap-2">
              <select value={printer ?? ""} onChange={e => setPrinter(e.target.value || null)} className="min-w-0 flex-1 rounded-xl border border-[#303030] bg-[#0d0d0d] px-3 py-2.5 text-sm text-white outline-none focus:border-[#4a8b3f]">
                <option value="">Select printer</option>
                {printers.map(p => <option key={p.name} value={p.name}>{p.name}{p.is_default ? " (Default)" : ""}</option>)}
              </select>
              <Button type="button" variant="secondary" onClick={() => void loadPrinters()} disabled={busy !== null}><RefreshCw className={`h-4 w-4 ${busy === "load" ? "animate-spin" : ""}`} /> Refresh</Button>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-3">
            <label className="text-xs font-medium text-[#aaa]">Paper
              <select value={paper} onChange={e => setPaper(e.target.value)} className="mt-1 w-full rounded-xl border border-[#303030] bg-[#0d0d0d] px-3 py-2.5 text-sm text-white outline-none focus:border-[#4a8b3f]"><option value="A4">A4</option><option value="LETTER">Letter</option></select>
            </label>
            <label className="text-xs font-medium text-[#aaa]">Orientation
              <select value={orientation} onChange={e => setOrientation(e.target.value as PrintOrientation)} className="mt-1 w-full rounded-xl border border-[#303030] bg-[#0d0d0d] px-3 py-2.5 text-sm text-white outline-none focus:border-[#4a8b3f]"><option value="portrait">Portrait</option><option value="landscape">Landscape</option></select>
            </label>
            <label className="text-xs font-medium text-[#aaa]">Copies
              <input type="number" min={1} max={99} value={copies} onChange={e => setCopies(Math.max(1, Math.min(99, Number(e.target.value) || 1)))} className="mt-1 w-full rounded-xl border border-[#303030] bg-[#0d0d0d] px-3 py-2.5 text-sm text-white outline-none focus:border-[#4a8b3f]" />
            </label>
          </div>

          <div className="rounded-xl border border-[#292929] bg-[#0d0d0d] px-3 py-2.5 text-[11px] text-[#888]">
            The report is generated from the real SQLite payroll data first. <span className="text-[#b5b5b5]">Print</span> sends that PDF to the selected printer; <span className="text-[#b5b5b5]">Save PDF</span> stores the same PDF in the configured output folder.
          </div>
        </div>

        <div className="flex justify-end gap-2 border-t border-[#292929] px-5 py-4">
          <Button type="button" variant="secondary" onClick={onClose} disabled={busy !== null}>Cancel</Button>
          <Button type="button" variant="secondary" onClick={() => void savePdf()} disabled={busy !== null}><Loader2 className={`h-4 w-4 ${busy === "pdf" ? "animate-spin" : "hidden"}`} /><span className={busy === "pdf" ? "hidden" : "inline"}>Save PDF</span><span className={busy === "pdf" ? "inline" : "hidden"}>Saving...</span></Button>
          <Button type="button" onClick={() => void print()} disabled={busy !== null || !printer}><Printer className="h-4 w-4" />{busy === "print" ? "Printing..." : "Print"}</Button>
        </div>
      </Card>
    </div>
  );
}
