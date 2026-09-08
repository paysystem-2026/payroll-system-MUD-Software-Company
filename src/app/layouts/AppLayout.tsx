import { Outlet } from "react-router-dom";
import { Sidebar } from "@/components/common/Sidebar";
import { Header } from "@/components/common/Header";
import { useAuthContext } from "@/stores/authContext";
import { WindowControls } from "@/components/common/WindowControls";
import { Suspense } from "react";
import { PageSkeleton } from "@/components/ui/PageSkeleton";

export function AppLayout() {
  const auth = useAuthContext();

  const handleLock = async () => {
    await auth.lock();
  };

  const handleLogout = async () => {
    await auth.logout();
  };

  return (
    <div className="app-desktop-shell flex h-screen min-h-[600px] flex-col overflow-hidden bg-[#0b0b0b] text-[#f5f5f5]">
      <div className="app-titlebar">
        <div className="app-titlebar-drag" data-tauri-drag-region>
          <span className="app-titlebar-title">Payroll System</span>
        </div>
        <WindowControls />
      </div>
      <div className="flex min-h-0 flex-1 overflow-hidden">
      <Sidebar />
      <div className="min-w-0 flex flex-1 flex-col overflow-hidden">
        <Header
          username={auth.status?.admin_username ?? null}
          onLock={handleLock}
          onLogout={handleLogout}
        />
        <main className="min-w-0 flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[1600px] p-5 lg:p-6 page-enter">
            <Suspense fallback={<PageSkeleton variant="dashboard" />}>
              <Outlet />
            </Suspense>
          </div>
        </main>
      </div>
      </div>
    </div>
  );
}
